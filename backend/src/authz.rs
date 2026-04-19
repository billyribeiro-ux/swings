//! FDN-07: Role-based access control policy engine.
//!
//! The runtime [`Policy`] is the in-memory projection of the two catalogue
//! tables introduced by migration `021_rbac.sql`:
//!
//! * `permissions` — the master list of permission keys (`<domain>.<resource>.<action>`)
//!   with human-readable descriptions.
//! * `role_permissions` — the join table that maps each [`UserRole`] to the set
//!   of permissions it is granted.
//!
//! A single [`Policy`] snapshot is loaded at application startup via
//! [`Policy::load`] and stored behind an [`Arc`] in [`crate::AppState`]. After
//! an admin mutates the role → permission mapping (out of scope for FDN-07;
//! handler lands in Round 2b), [`Policy::reload`] refreshes the in-memory
//! view without restarting the process.
//!
//! # Intended use
//!
//! ```ignore
//! async fn admin_dashboard(
//!     State(state): State<AppState>,
//!     auth: AuthUser,
//! ) -> AppResult<Json<Dashboard>> {
//!     state.policy.require(&auth, "admin.dashboard.read")?;
//!     // …
//! }
//! ```
//!
//! The decision is strictly based on role → permission membership; row-level
//! checks (e.g. "blog.post.update_own" vs "blog.post.update_any") are the
//! caller's responsibility.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use sqlx::PgPool;

use crate::error::AppError;
use crate::extractors::AuthUser;
use crate::models::UserRole;

// ── Hot-swap handle ────────────────────────────────────────────────────

/// Atomic-swap container around a [`Policy`] snapshot. Lives inside
/// [`crate::AppState::policy`] so the role-matrix admin handler can
/// reload the in-memory matrix immediately after a write.
///
/// Read path: takes the read-lock for the duration of one `Arc::clone`
/// (microseconds), then drops the guard before any user code runs.
/// The cloned `Arc<Policy>` keeps the snapshot alive even while a
/// concurrent writer swaps in a fresh one — readers in flight finish
/// against the snapshot they captured.
///
/// Lock-poisoning recovery: if a writer panicked while holding the
/// write-lock, subsequent readers recover by `into_inner`-ing the
/// poison. The runtime invariant we care about (the inner `Arc<Policy>`
/// is always valid) is upheld by construction — there is no
/// intermediate state in `replace` that could leave the cell in a
/// broken shape.
#[derive(Debug)]
pub struct PolicyHandle {
    inner: RwLock<Arc<Policy>>,
}

impl PolicyHandle {
    /// Build a fresh handle around an in-memory policy.
    #[must_use]
    pub fn new(policy: Policy) -> Self {
        Self {
            inner: RwLock::new(Arc::new(policy)),
        }
    }

    /// Load the current policy snapshot. Cheap (one `Arc::clone`).
    pub fn snapshot(&self) -> Arc<Policy> {
        match self.inner.read() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }

    /// Atomically replace the active policy. Existing readers complete
    /// against the previous snapshot; new readers see the new one.
    pub fn replace(&self, policy: Policy) {
        let new_arc = Arc::new(policy);
        match self.inner.write() {
            Ok(mut g) => *g = new_arc,
            Err(p) => *p.into_inner() = new_arc,
        }
    }

    /// Reload from the database and swap in the result. Returns the
    /// number of (role, permission) pairs in the new snapshot so the
    /// caller can assert the catalogue actually populated.
    pub async fn reload_from_db(&self, pool: &PgPool) -> Result<usize, AppError> {
        let next = Policy::load(pool).await?;
        let count = next.len();
        self.replace(next);
        Ok(count)
    }

    /// Convenience predicate forwarded to the current snapshot.
    pub fn has(&self, role: UserRole, perm: &str) -> bool {
        self.snapshot().has(role, perm)
    }

    /// Convenience requirement forwarded to the current snapshot.
    pub fn require(&self, auth: &AuthUser, perm: &str) -> Result<(), AppError> {
        self.snapshot().require(auth, perm)
    }
}

/// Immutable snapshot of the `role → permission` matrix.
///
/// The struct is cheap to clone behind an [`Arc`]; [`Policy::reload`] takes an
/// `&self` receiver and returns a new [`Arc<Policy>`] so the caller can swap
/// it into shared state atomically without readers observing a partial update.
#[derive(Debug, Clone, Default)]
pub struct Policy {
    perms_by_role: HashMap<UserRole, HashSet<String>>,
}

impl Policy {
    /// Build a policy from an explicit in-memory fixture. Primarily useful for
    /// tests and documentation examples — production code should call
    /// [`Policy::load`] to hydrate from the database.
    #[must_use]
    pub fn from_pairs<I, K>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (UserRole, K)>,
        K: Into<String>,
    {
        let mut perms_by_role: HashMap<UserRole, HashSet<String>> = HashMap::new();
        for (role, perm) in pairs {
            perms_by_role.entry(role).or_default().insert(perm.into());
        }
        Self { perms_by_role }
    }

    /// Hydrate a [`Policy`] from the `role_permissions` table.
    ///
    /// Returns [`AppError::Database`] on transport or schema errors and
    /// [`AppError::Internal`] if a row carries an unknown role string — the
    /// enum and the seed are generated from the same source of truth so this
    /// only fires if the DB has been edited by hand.
    pub async fn load(pool: &PgPool) -> Result<Self, AppError> {
        // We fetch `role::text` rather than relying on `sqlx::Type` on the
        // client side because the policy loader runs early at startup and we
        // want a defensive fallback path if the enum grows before the binary
        // is redeployed.
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT role::text, permission FROM role_permissions")
                .fetch_all(pool)
                .await?;

        let mut perms_by_role: HashMap<UserRole, HashSet<String>> = HashMap::new();
        for (role_str, perm) in rows {
            let role = UserRole::from_str_lower(&role_str).ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "role_permissions contains unknown role: {role_str}"
                ))
            })?;
            perms_by_role.entry(role).or_default().insert(perm);
        }

        Ok(Self { perms_by_role })
    }

    /// Reload the policy from the database and return a fresh [`Arc<Policy>`].
    ///
    /// Callers swap the returned `Arc` into shared state; existing readers on
    /// the previous snapshot finish their request on the old view without
    /// contention.
    pub async fn reload(pool: &PgPool) -> Result<Arc<Self>, AppError> {
        Ok(Arc::new(Self::load(pool).await?))
    }

    /// Returns `true` if `role` has been granted `permission`.
    #[must_use]
    pub fn has(&self, role: UserRole, permission: &str) -> bool {
        self.perms_by_role
            .get(&role)
            .is_some_and(|set| set.contains(permission))
    }

    /// Require that `auth`'s role carries `permission`; return
    /// [`AppError::Forbidden`] otherwise.
    ///
    /// `auth.role` is the stringly-typed JWT claim; if it does not map to a
    /// known [`UserRole`] the request is rejected with
    /// [`AppError::Unauthorized`] (the client's bearer token is presenting
    /// an obsolete or forged role).
    pub fn require(&self, auth: &AuthUser, permission: &str) -> Result<(), AppError> {
        let role = UserRole::from_str_lower(&auth.role).ok_or(AppError::Unauthorized)?;
        if self.has(role, permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    /// Iterate over every (role, permission) pair currently in the snapshot.
    /// Used by tests + future admin UIs.
    pub fn iter(&self) -> impl Iterator<Item = (UserRole, &str)> + '_ {
        self.perms_by_role
            .iter()
            .flat_map(|(role, perms)| perms.iter().map(move |p| (*role, p.as_str())))
    }

    /// Total (role, permission) entries — handy in assertions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.perms_by_role.values().map(HashSet::len).sum()
    }

    /// `true` if no pairs have been loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.perms_by_role.values().all(HashSet::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sample_policy() -> Policy {
        Policy::from_pairs([
            (UserRole::Member, "user.self.read"),
            (UserRole::Member, "order.mine.read"),
            (UserRole::Author, "user.self.read"),
            (UserRole::Author, "blog.post.create"),
            (UserRole::Author, "blog.post.update_own"),
            (UserRole::Support, "user.self.read"),
            (UserRole::Support, "order.any.read"),
            (UserRole::Support, "order.refund.create"),
            (UserRole::Admin, "user.self.read"),
            (UserRole::Admin, "blog.post.update_any"),
            (UserRole::Admin, "admin.role.manage"),
        ])
    }

    fn auth(role: &str) -> AuthUser {
        AuthUser {
            user_id: Uuid::new_v4(),
            role: role.to_string(),
            impersonator_id: None,
            impersonator_role: None,
            impersonation_session_id: None,
        }
    }

    #[test]
    fn has_matches_granted_permissions() {
        let p = sample_policy();
        assert!(p.has(UserRole::Member, "user.self.read"));
        assert!(p.has(UserRole::Author, "blog.post.create"));
        assert!(p.has(UserRole::Support, "order.refund.create"));
        assert!(p.has(UserRole::Admin, "admin.role.manage"));
    }

    #[test]
    fn has_rejects_ungranted_permissions() {
        let p = sample_policy();
        assert!(!p.has(UserRole::Member, "blog.post.create"));
        assert!(!p.has(UserRole::Author, "admin.role.manage"));
        assert!(!p.has(UserRole::Support, "blog.post.update_any"));
        assert!(!p.has(UserRole::Admin, "nonexistent.permission"));
    }

    #[test]
    fn has_is_false_for_unknown_role_in_map() {
        let mut p = Policy::default();
        // member grants exist but nothing is assigned to author/admin.
        p.perms_by_role
            .entry(UserRole::Member)
            .or_default()
            .insert("x".into());
        assert!(p.has(UserRole::Member, "x"));
        assert!(!p.has(UserRole::Admin, "x"));
    }

    #[test]
    fn require_ok_when_role_has_permission() {
        let p = sample_policy();
        let res = p.require(&auth("admin"), "admin.role.manage");
        assert!(res.is_ok());
    }

    #[test]
    fn require_forbidden_when_role_lacks_permission() {
        let p = sample_policy();
        match p.require(&auth("member"), "admin.role.manage") {
            Err(AppError::Forbidden) => {}
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }

    #[test]
    fn require_unauthorized_when_role_string_unrecognized() {
        let p = sample_policy();
        match p.require(&auth("root"), "admin.role.manage") {
            Err(AppError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn member_vs_admin_matrix() {
        let p = sample_policy();
        // Positive samples
        assert!(p.require(&auth("member"), "user.self.read").is_ok());
        assert!(p.require(&auth("admin"), "user.self.read").is_ok());
        assert!(p.require(&auth("admin"), "blog.post.update_any").is_ok());

        // Negative samples — admin-only permission denied to member
        assert!(matches!(
            p.require(&auth("member"), "blog.post.update_any"),
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            p.require(&auth("member"), "admin.role.manage"),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn iter_yields_every_pair() {
        let p = sample_policy();
        let collected: HashSet<(UserRole, String)> =
            p.iter().map(|(r, s)| (r, s.to_string())).collect();
        assert_eq!(collected.len(), p.len());
        assert!(collected.contains(&(UserRole::Admin, "admin.role.manage".into())));
        assert!(collected.contains(&(UserRole::Member, "user.self.read".into())));
    }

    #[test]
    fn empty_policy_rejects_everything() {
        let p = Policy::default();
        assert!(p.is_empty());
        assert!(!p.has(UserRole::Admin, "anything"));
        assert!(matches!(
            p.require(&auth("admin"), "anything"),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn from_pairs_dedupes() {
        let p = Policy::from_pairs([
            (UserRole::Member, "x"),
            (UserRole::Member, "x"),
            (UserRole::Member, "y"),
        ]);
        assert_eq!(p.len(), 2);
        assert!(p.has(UserRole::Member, "x"));
        assert!(p.has(UserRole::Member, "y"));
    }
}
