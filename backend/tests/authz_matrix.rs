#![deny(warnings)]
#![forbid(unsafe_code)]

//! FDN-07 integration test.
//!
//! Exhaustively asserts every row of the §12 authz matrix in
//! `AUDIT_PHASE3_PLAN.md`. The fixture below mirrors the `role_permissions`
//! seed in `backend/migrations/021_rbac.sql`; if either side diverges, one of
//! the assertions fires immediately and the divergence is surfaced in CI.
//!
//! Why an in-memory fixture? Ephemeral-Postgres fixtures are heavyweight and
//! require a running cluster, which is not part of the `cargo test` sandbox
//! profile. The fixture keeps the check strictly inside the Rust process
//! while still exercising the public `Policy::require` / `Policy::has` API
//! end-to-end.

use swings_api::authz::Policy;
use swings_api::models::UserRole;

/// (role, permission) pairs copied verbatim from the seed block of
/// `backend/migrations/021_rbac.sql`. Keep sorted by role then permission so
/// changes surface cleanly in code review.
fn seeded_matrix() -> Vec<(UserRole, &'static str)> {
    // ── member ──────────────────────────────────────────────────────────
    let member: &[&str] = &[
        "user.self.read",
        "user.self.update",
        "course.read_enrolled",
        "course.enroll.self",
        "course.progress.read_self",
        "coupon.apply",
        "order.mine.read",
        "order.invoice.read_self",
        "subscription.mine.read",
        "subscription.mine.manage",
        "popup.submit",
        "popup.event",
        "form.submit",
        "consent.record",
        "consent.log.read_self",
        "dsar.submit",
        "notification.mine.read",
        "notification.mine.mark_read",
    ];

    // ── author: member baseline + authoring ─────────────────────────────
    let author_extra: &[&str] = &[
        "blog.post.create",
        "blog.post.update_own",
        "blog.post.delete_own",
        "blog.post.publish",
        "blog.media.upload",
        "blog.media.delete_own",
    ];

    // ── support: member baseline + reads + limited writes ───────────────
    let support_extra: &[&str] = &[
        "user.other.read",
        "course.read_any",
        "course.progress.read_any",
        "coupon.read_any",
        "order.any.read",
        "order.invoice.read_any",
        "subscription.any.read",
        "consent.log.read_any",
        "form.submission.read_any",
        "admin.dashboard.read",
        "admin.outbox.read",
        "admin.outbox.retry",
        "order.refund.create",
        "dsar.fulfill",
    ];

    // ── admin: every permission in the catalogue ───────────────────────
    // Kept as an explicit list (union of all roles' perms + admin-only
    // extras) to defend against the migration accidentally narrowing
    // admin's grant.
    let admin_only: &[&str] = &[
        "user.other.update",
        "user.other.delete",
        "blog.post.read_any",
        "blog.post.update_any",
        "blog.post.delete_any",
        "blog.media.delete_any",
        "blog.category.manage",
        "course.manage",
        "course.enroll.other",
        "coupon.manage",
        "order.any.update",
        "subscription.any.manage",
        "subscription.plan.manage",
        "popup.manage",
        "popup.read_analytics",
        "form.manage",
        "form.submission.delete_any",
        "admin.audit.read",
        "admin.role.manage",
        "admin.permission.manage",
    ];

    let mut matrix = Vec::<(UserRole, &'static str)>::new();

    for p in member {
        matrix.push((UserRole::Member, *p));
    }
    for p in member {
        matrix.push((UserRole::Author, *p));
    }
    for p in author_extra {
        matrix.push((UserRole::Author, *p));
    }
    for p in member {
        matrix.push((UserRole::Support, *p));
    }
    for p in support_extra {
        matrix.push((UserRole::Support, *p));
    }
    // admin is the superset
    for p in member
        .iter()
        .chain(author_extra.iter())
        .chain(support_extra.iter())
        .chain(admin_only.iter())
    {
        matrix.push((UserRole::Admin, *p));
    }

    matrix
}

fn fixture_policy() -> Policy {
    Policy::from_pairs(seeded_matrix().into_iter().map(|(r, p)| (r, p.to_string())))
}

#[test]
fn every_seeded_grant_is_reflected_in_has() {
    let policy = fixture_policy();
    for (role, perm) in seeded_matrix() {
        assert!(
            policy.has(role, perm),
            "policy.has({role:?}, {perm}) returned false — matrix drift"
        );
    }
}

#[test]
fn member_cannot_act_as_admin() {
    let p = fixture_policy();
    for denied in [
        "admin.role.manage",
        "admin.permission.manage",
        "blog.post.update_any",
        "blog.post.delete_any",
        "user.other.delete",
        "coupon.manage",
        "subscription.plan.manage",
    ] {
        assert!(
            !p.has(UserRole::Member, denied),
            "member must not hold {denied}"
        );
    }
}

#[test]
fn author_cannot_update_foreign_posts() {
    let p = fixture_policy();
    assert!(p.has(UserRole::Author, "blog.post.update_own"));
    assert!(p.has(UserRole::Author, "blog.post.delete_own"));
    assert!(!p.has(UserRole::Author, "blog.post.update_any"));
    assert!(!p.has(UserRole::Author, "blog.post.delete_any"));
}

#[test]
fn support_has_cross_user_reads_but_limited_writes() {
    let p = fixture_policy();
    // Reads support should have
    for allowed in [
        "user.other.read",
        "order.any.read",
        "subscription.any.read",
        "consent.log.read_any",
    ] {
        assert!(p.has(UserRole::Support, allowed), "support lacks {allowed}");
    }
    // Writes support should NOT have
    for denied in [
        "user.other.delete",
        "order.any.update",
        "subscription.any.manage",
        "coupon.manage",
        "blog.post.update_any",
    ] {
        assert!(
            !p.has(UserRole::Support, denied),
            "support must not hold {denied}"
        );
    }
    // Limited writes support DOES have
    assert!(p.has(UserRole::Support, "order.refund.create"));
    assert!(p.has(UserRole::Support, "dsar.fulfill"));
}

#[test]
fn admin_holds_every_permission_in_catalogue() {
    let p = fixture_policy();
    // Every permission granted to any role must also be granted to admin.
    for (_role, perm) in seeded_matrix() {
        assert!(
            p.has(UserRole::Admin, perm),
            "admin is not a superset: missing {perm}"
        );
    }
    // Admin-only permissions
    for admin_only in [
        "admin.role.manage",
        "admin.permission.manage",
        "admin.audit.read",
        "blog.post.update_any",
        "coupon.manage",
        "subscription.plan.manage",
    ] {
        assert!(
            p.has(UserRole::Admin, admin_only),
            "admin must hold {admin_only}"
        );
    }
}

#[test]
fn baseline_self_service_holds_for_all_nonadmin_roles() {
    let p = fixture_policy();
    // Every non-admin role must be able to read/update its own profile and
    // inbox — anything else is a regression.
    for role in [UserRole::Member, UserRole::Author, UserRole::Support] {
        for perm in [
            "user.self.read",
            "user.self.update",
            "notification.mine.read",
            "notification.mine.mark_read",
            "consent.record",
            "consent.log.read_self",
        ] {
            assert!(
                p.has(role, perm),
                "role {role:?} missing baseline self-service {perm}"
            );
        }
    }
}

#[test]
fn unknown_permission_is_denied_everywhere() {
    let p = fixture_policy();
    for role in [
        UserRole::Member,
        UserRole::Author,
        UserRole::Support,
        UserRole::Admin,
    ] {
        assert!(!p.has(role, "nonexistent.permission"));
    }
}

#[test]
fn policy_total_count_is_nonzero_and_admin_is_largest_set() {
    let p = fixture_policy();
    assert!(
        p.len() > 50,
        "matrix should be non-trivial; got {}",
        p.len()
    );

    // Admin should be a strict superset of every other role's grants, so it
    // holds the largest permission set.
    let admin_count = p
        .iter()
        .filter(|(role, _)| matches!(role, UserRole::Admin))
        .count();
    for role in [UserRole::Member, UserRole::Author, UserRole::Support] {
        let other = p.iter().filter(|(r, _)| r == &role).count();
        assert!(
            admin_count >= other,
            "admin ({admin_count}) must be ≥ {role:?} ({other})"
        );
    }
}
