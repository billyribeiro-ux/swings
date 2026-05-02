# Auth Graph — Phase 1 Surface Map

Inventory of every authorization construct observed in the admin attack surface. Counts derived from `routes.json`.

---

## Construct: `AdminUser` extractor

- **Defined:** `backend/src/extractors.rs:311` (struct), `backend/src/extractors.rs:352` (`FromRequestParts` impl).
- **Semantics:** Strict — only the `admin` JWT role passes (extractors.rs:379). Impersonation tokens are rejected with `Forbidden` (extractors.rs:368).
- **Helper methods:** `has_permission` (extractors.rs:330), `require` (extractors.rs:342).
- **Used by routes (counted):** 189 routes — every `/api/admin/*` route except those gated by `PrivilegedUser`. See `routes.json` filter `extractors contains "AdminUser"`.

## Construct: `PrivilegedUser` extractor

- **Defined:** `backend/src/extractors.rs:397` (struct), `backend/src/extractors.rs:415` (impl).
- **Semantics:** Permits any role carrying `admin.dashboard.read` (admin always; support per the `021_rbac.sql` seed). Rejects impersonation tokens (extractors.rs:425).
- **Used by routes (34):** R-0023..R-0027 (admin_security: suspend/unsuspend/reactivate/ban/unban/force-pw/verify-email/sessions etc.), R-0035..R-0036 (admin_members), R-0046..R-0051 (admin_orders), R-0052..R-0058 (admin_dsar), R-0061..R-0063 (admin_audit), R-0036..R-0041 (admin_subscriptions).

## Construct: `AuthUser` extractor

- **Defined:** `backend/src/extractors.rs:76` (struct), `backend/src/extractors.rs:203` (impl).
- **Semantics:** Authenticated user (any role). Bounces banned/suspended users (extractors.rs:236-258). Honours impersonation token revocation (extractors.rs:265-298).
- **Used by routes (36):** all `/api/member/*` routes, `/api/auth/me`, `/api/auth/logout`, `/api/coupons/apply`, `/api/cart/merge`, `/api/auth/impersonation/exit`, `/api/member/courses/*` (enroll/progress).

## Construct: `MaybeAuthUser` (infallible) — extractors.rs:519

Used on logout-style endpoints; never used to gate admin actions. Routes: 2 (logout-related).

## Construct: `OptionalAuthUser` (infallible) — extractors.rs:537

Public surfaces that personalise on identity. 12 routes in catalog/cart/popups/forms/consent.

## Construct: `PaidUser` — extractors.rs:460

Subscription-gated. 1 route: `PUT /api/member/lessons/:lesson_id/progress` (R-0263).

## Construct: `RoleUser` — extractors.rs:590

Typed-role variant of `AuthUser`. **Not observed in any handler signature** in the current admin surface. Reserved for FDN-07 Round 2b migration per the docstring.

---

## Construct: `policy.require(ctx, "<perm>")` / `admin.require(&state.policy, "<perm>")` / `privileged.require(...)`

- **Defined:** `backend/src/authz.rs:76` (`Policy::require`), `backend/src/extractors.rs:342` (`AdminUser::require`), `backend/src/extractors.rs:406` (`PrivilegedUser::require`).
- **Permission catalogue source:** `021_rbac.sql` and the `0NN_*_perms.sql` family (per AGENTS.md §6).
- **Distinct permission strings observed in handler bodies (67 — string-literal grep):**

```
admin.audit.export, admin.audit.read, admin.impersonate, admin.ip_allowlist.manage,
admin.ip_allowlist.read, admin.member.billing_portal, admin.member.create,
admin.member.delete, admin.member.read, admin.member.role.update,
admin.member.subscription.manage, admin.member.update, admin.outbox.read,
admin.outbox.retry, admin.role.manage, admin.role.read, admin.security.read,
admin.settings.read, admin.settings.read_secret, admin.settings.write,
admin.subscription.comp, admin.subscription.cycle, admin.subscription.extend,
admin.subscription.read, admin.watchlist.alert.manage, admin.watchlist.alert.read,
admin.watchlist.manage, admin.watchlist.read,
blog.category.manage, blog.media.delete_any, blog.media.upload,
blog.post.create, blog.post.delete_any, blog.post.publish, blog.post.update_any,
consent.config.manage, consent.log.read_any,
coupon.manage, coupon.read_any,
course.manage,
dsar.erase.approve, dsar.erase.request, dsar.export, dsar.read_any,
form.submission.export, form.submission.read_any,
notification.broadcast.create, notification.template.manage,
order.any.export, order.any.read, order.manual.create, order.refund.create,
order.void.create,
popup.manage, popup.read_analytics,
product.manage,
subscription.plan.manage, subscription.price_protection.manage,
user.ban, user.email.verify, user.force_password_reset, user.reactivate,
user.session.read, user.session.revoke, user.suspend
```

- **Routes carrying at least one `*.require(...)` call:** ~167 (every admin-surface mutation + most reads). See per-route `policy_require_calls` field in `routes.json`.

---

## Construct: middleware-level gates

### `admin_ip_allowlist::enforce` (per-mount layer)

- **Defined:** `backend/src/middleware/admin_ip_allowlist.rs`. Lookup helper `backend/src/security/ip_allowlist.rs`.
- **Mount:** `backend/src/main.rs:701`. Applies once to the entire `admin_routes` Router so every `/api/admin/*` route inherits.
- **Semantics:** Fails open on empty `admin_ip_allowlist` table; fails closed on non-empty (per AGENTS.md §6).

### `rate_limit::admin_mutation_rate_limit` (per-mount layer)

- **Defined:** `backend/src/middleware/rate_limit.rs`.
- **Mount:** `backend/src/main.rs:697`. Token-bucket per actor for `POST/PUT/PATCH/DELETE`.

### `idempotency::enforce` (per-router layer; required by AGENTS.md hard rule 6 for admin POSTs)

- **Defined:** `backend/src/middleware/idempotency.rs`.
- **Mounts:** see `middleware-chain.md` table.

---

## Routes with NO server-side auth construct in the route's effective chain

For every `/api/admin/*` route the `AdminUser`/`PrivilegedUser` extractor sits in the handler signature, so all 196 admin-tree routes are gated. Below is the explicit list of routes in `routes.json` whose handler does **not** declare an authentication extractor and lacks any permission-gating middleware in its effective chain (excluding routes that are intentionally public — login/forgot/etc. — but listed for completeness):

### Public-by-design (no auth extractor by intent — kept for inventory completeness)

- R-0240 `POST /api/auth/register`
- R-0241 `POST /api/auth/login`
- R-0242 `POST /api/auth/forgot-password`
- R-0243 `POST /api/auth/refresh`
- R-0246 `POST /api/auth/reset-password`
- R-0247 `POST /api/auth/verify-email`
- R-0248 `POST /api/auth/resend-verification`
- R-0251 `POST /api/analytics/events`
- R-0252..R-0259 `/api/blog/*` (public)
- R-0260..R-0261 `/api/courses/*` (public)
- R-0264 `GET /api/pricing/plans`
- R-0265..R-0266 `/api/coupons/*` public
- R-0267..R-0270 `/api/popups/*` public
- R-0271..R-0272 `/api/products/*` public
- R-0273..R-0279 `/api/forms/*` public
- R-0280..R-0282 `/api/catalog/*`
- R-0283..R-0288 `/api/cart/*`
- R-0289 `GET /api/consent/banner`
- R-0290 `GET /api/consent/me`
- R-0291 `POST /api/consent/record`
- R-0292 `POST /api/dsar`
- R-0318 `POST /api/webhooks/stripe` — gated by HMAC signature inside handler (webhooks.rs:173 `verify_stripe_signature`).
- R-0319 `POST /api/webhooks/email/resend` — gated by HMAC signature.
- R-0320 `POST /api/csp-report`
- R-0321 `GET /u/unsubscribe`
- R-0322..R-0324 `/health`, `/ready`, `/version`
- R-0325 `GET /metrics` — admin-gated in production (per main.rs:770), public in dev.
- R-0327 `POST /api/greeks-pdf` (SvelteKit) — no auth, no rate-limit, body validation only.

### Admin-tree routes whose handler signature lacks a `*.require(...)` call (extractor-only, no fine-grained perm)

The following admin routes have `AdminUser` in the extractor row but **no observed `policy_require_calls` entry** (i.e. the handler relies on the coarse "role == admin" check from the `AdminUser` extractor without a per-action permission). This matches the open-item `O-1` flagged in `CLAUDE.md`. The list is mechanical (handler grep, not opinion):

- R-0001 `GET /api/admin/stats`
- R-0002 `GET /api/admin/analytics/summary`
- R-0003 `GET /api/admin/analytics/revenue`
- R-0004 `GET /api/admin/members`
- R-0005 `GET /api/admin/members/:id`
- R-0008 `GET /api/admin/members/:id/subscription`
- R-0064 `GET /api/admin/blog/posts`
- R-0066 `GET /api/admin/blog/posts/:id`
- R-0072 `GET /api/admin/blog/posts/:id/revisions`
- R-0074 `GET /api/admin/blog/posts/:id/meta`
- R-0077 `GET /api/admin/blog/categories`
- R-0081 `GET /api/admin/blog/tags`
- R-0084 `GET /api/admin/blog/media`
- R-0089 `GET /api/admin/courses`
- R-0091 `GET /api/admin/courses/:id`
- R-0102 `GET /api/admin/pricing/plans`
- R-0104 `GET /api/admin/pricing/plans/price-log`
- R-0105 `GET /api/admin/pricing/plans/:id`
- R-0109 `GET /api/admin/pricing/plans/:id/history`
- R-0112 `GET /api/admin/coupons`
- R-0115 `GET /api/admin/coupons/stats`
- R-0120 `GET /api/admin/coupons/:id/usages`
- R-0122 `GET /api/admin/popups`
- R-0125 `GET /api/admin/popups/:id`
- R-0130 `GET /api/admin/popups/:id/submissions`
- R-0133 `GET /api/admin/products`
- R-0135 `GET /api/admin/products/:id`
- R-0139 `GET /api/admin/products/:id/variants`
- R-0143 `GET /api/admin/products/:id/assets`
- R-0146 `GET /api/admin/products/:id/bundle-items`
- R-0152 `GET /api/admin/notifications/templates`
- R-0154 `GET /api/admin/notifications/templates/:id`
- R-0158 `GET /api/admin/notifications/deliveries`
- R-0159 `GET /api/admin/notifications/suppression`
- R-0167 `GET /api/admin/consent/banners`
- R-0169 `GET /api/admin/consent/banners/:id`
- R-0171 `GET /api/admin/consent/categories`
- R-0174 `GET /api/admin/consent/services`
- R-0177 `GET /api/admin/consent/policies`
- R-0182 `GET /api/admin/consent/dsar`
- R-0183 `POST /api/admin/consent/dsar/:id/fulfill`

(All listed routes are gated by `AdminUser` strict `role == admin`; the gap is "no fine-grained `policy.require(...)` for support-role expansion". Phase 1 is inventory, not severity.)
