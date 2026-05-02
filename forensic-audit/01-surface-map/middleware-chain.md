# Middleware Chain — Phase 1 Surface Map

Each section lists the EXACT outermost-to-innermost layer order applied to the routes mounted under that path. Mounts are derived from `backend/src/main.rs`. Layer order reads OUTER → INNER (request hits layer 1 first, then 2, then …).

The two top-level layers that wrap **every** request reside on the root `app` router (main.rs:777-802):

```
Outer-most (applied last via .layer):
  Extension(metrics_handle)                       (main.rs:778)
  observability::metrics::http_middleware          (main.rs:779)
  observability::correlation::middleware           (main.rs:783)
  middleware::impersonation_banner::stamp          (main.rs:789)
  middleware::maintenance_mode::enforce            (main.rs:796)
  CorsLayer                                        (main.rs:800)
  TraceLayer::new_for_http                         (main.rs:801)
```

Below, "global stack" means those seven layers always apply on top.

---

## Mount: `/api/admin/*` (the entire admin tree, main.rs:498-719)

The `admin_routes` Router carries TWO outermost layers shared by every nested admin sub-router:

```
1. middleware::admin_ip_allowlist::enforce              (main.rs:716)
2. middleware::rate_limit::admin_mutation_rate_limit    (main.rs:712)
```

Then EACH `.nest(...)`/`.merge(...)` call applies its own `idempotency::enforce` layer (main.rs:511-707), so the per-route innermost-first chain is:

```
3. middleware::idempotency::enforce                     (per-mount layer, see below)
4. (extractor) AdminUser | PrivilegedUser               (in handler signature)
5. (handler-internal) admin.require(state.policy, "<perm>")
6. (handler-internal) audit_admin / audit_admin_priv / record_admin_action
```

Per-mount idempotency layer source lines:

| Mount sub-prefix                             | Nest line                                                       | Idempotency layer wrap  |
| -------------------------------------------- | --------------------------------------------------------------- | ----------------------- |
| `/api/admin` (legacy admin::router)          | main.rs:505                                                     | main.rs:511             |
| `/api/admin` (admin_security via .merge)     | main.rs:520                                                     | main.rs:521 (per merge) |
| `/api/admin/members` (admin_members .merge)  | main.rs:532                                                     | main.rs:533             |
| `/api/admin/security/ip-allowlist`           | main.rs:540                                                     | main.rs:541             |
| `/api/admin/security/impersonation`          | main.rs:551                                                     | main.rs:553             |
| `/api/admin/settings`                        | main.rs:564                                                     | main.rs:566             |
| `/api/admin/security/roles`                  | main.rs:573                                                     | main.rs:574             |
| `/api/admin/subscriptions`                   | main.rs:585                                                     | main.rs:587             |
| `/api/admin/orders`                          | main.rs:598                                                     | main.rs:600             |
| `/api/admin/dsar`                            | main.rs:609                                                     | main.rs:611             |
| `/api/admin/audit`                           | main.rs:616 — **NO** layer (lacks `.layer(...)` after the nest) |
| `/api/admin/blog`                            | main.rs:626                                                     | main.rs:628             |
| `/api/admin/courses`                         | main.rs:633                                                     | main.rs:635             |
| `/api/admin/pricing`                         | main.rs:640                                                     | main.rs:642             |
| `/api/admin/coupons`                         | main.rs:647                                                     | main.rs:649             |
| `/api/admin/popups`                          | main.rs:654                                                     | main.rs:656             |
| `/api/admin/products`                        | main.rs:661                                                     | main.rs:663             |
| `/api/admin/outbox`                          | main.rs:668                                                     | main.rs:670             |
| `/api/admin/forms`                           | main.rs:675                                                     | main.rs:677             |
| `/api/admin/notifications`                   | main.rs:682                                                     | main.rs:684             |
| `/api/admin/consent` (admin_consent)         | main.rs:691                                                     | main.rs:693             |
| `/api/admin/consent` (consent::admin_router) | main.rs:700 — **DUPLICATE PREFIX** of admin_consent             | main.rs:702             |

### Flag — duplicate prefix on `/api/admin/consent`

Two distinct sub-routers are nested under the same prefix at lines 675-690 of `main.rs`. They register disjoint route sets:

- `admin_consent::router()` (CONSENT-07 — banners/categories/services/policies/log/integrity).
- `consent::admin_router()` (CONSENT-03 — DSAR list + fulfill).

Both apply their own `idempotency::enforce` layer; this means a consent DSAR request transits **both** the IP allowlist + rate-limit (shared, top-level) and **its own** idempotency middleware, while a consent-banner request transits the same outer layers and the OTHER nest's idempotency middleware. Because each nest's set of route-paths is disjoint, axum routes deterministically — but the doubled-mount evidence is preserved in `routes.json` (see `R-0167`–`R-0181` admin_consent vs `R-0182`–`R-0183` consent admin_router).

### Flag — `/api/admin/audit` lacks idempotency layer

Inside `admin_routes`, every `.nest(...)` call wires `axum::middleware::from_fn_with_state(state.clone(), idempotency::enforce)` EXCEPT line 616:

```
.nest("/audit", handlers::admin_audit::router()),
```

There is no `.layer(...)` call after this nest. The siblings on either side (`/dsar` at 609, `/blog` at 626) both wrap the layer. Routes affected: R-0061, R-0062, R-0063 (`GET /api/admin/audit*` — read-only, but the layer is still missing for parity).

---

## Mount: `/api/auth` (main.rs:727)

No global rate-limit; per-route `.merge(... .layer(...))` for the three sensitive endpoints:

```
- /register          → rate_limit::register_layer        (auth.rs:149)
- /login             → rate_limit::login_layer           (auth.rs:154)
- /forgot-password   → rate_limit::forgot_password_layer (auth.rs:159)
- /refresh, /me, /logout, /reset-password, /verify-email,
  /resend-verification → no per-route layer
```

Routes: R-0240..R-0249.

## Mount: `/api/auth/impersonation` (main.rs:731)

No middleware. Single route `/exit` (R-0250). The route's purpose is to let an impersonated session call out without going through the IP-allowlist-gated `/api/admin/*` tree (per main.rs:713-718 docstring).

## Mount: `/api/analytics` (main.rs:720)

No layer. Single route. (R-0251)

## Mount: `/api/blog` (main.rs:723) — public

No layer. Routes: R-0252..R-0259.

## Mount: `/api/courses` (main.rs:724) — public

No layer. R-0260..R-0261.

## Mount: `/api/pricing` (main.rs:725) — public

No layer. R-0264.

## Mount: `/api/coupons` (main.rs:726) — public

`/apply` carries `rate_limit::coupon_apply_layer` (coupons.rs:58). `/validate` is unlimited. R-0265..R-0266.

## Mount: `/api/popups` (main.rs:727) — public

```
- /event   → rate_limit::popup_event_layer  (popups.rs:59)
- /submit  → rate_limit::popup_submit_layer (popups.rs:64)
```

R-0267..R-0270.

## Mount: `/api/products` (main.rs:728) — public

No layer. R-0271..R-0272.

## Mount: `/api/forms` (main.rs:729) — public

```
- /{slug}/submit, /{slug}/payment-intent → rate_limit::form_submit_layer  (forms.rs:60, 73)
- /{slug}/partial (POST + GET)           → rate_limit::form_partial_layer (forms.rs:66)
- /{slug}, /geo/*                        → no per-route layer
```

R-0273..R-0279.

## Mount: `/api/catalog` (main.rs:731) — public

No layer. R-0280..R-0282.

## Mount: `/api/cart` (main.rs:733)

No layer. Authenticated identity is optional via `OptionalAuthUser`. R-0283..R-0288.

## Mount: `/api/consent` (main.rs:735) and `/api/dsar` (main.rs:736) — public

`/record` carries `rate_limit::consent_record_layer` (consent.rs:62 — internal merge). Others unlimited.

R-0289..R-0292.

## Mount: `/api/member` (main.rs:744)

```
1. middleware::idempotency::enforce          (main.rs:746)
2. middleware::rate_limit::member_layer       (member.rs:82)
3. AuthUser extractor                         (per handler)
```

Per the audit map, member self-service POST/DELETE handlers also call `audit_admin_under_impersonation(...)` so admin actions performed via impersonation tokens get audit-logged.

R-0293..R-0317.

The same prefix also accepts a SECOND `.nest("/api/member", handlers::courses::member_router())` (main.rs:751) and a THIRD `.nest("/api/member", handlers::notifications::member_router())` (main.rs:752). These do NOT carry the idempotency / rate-limit layers — they are sibling-nested and the layer wrap from line 746 only applies to the first nest.

### Flag — multi-nest on `/api/member` with asymmetric middleware

| Mount line  | Router source                          | idempotency | rate_limit::member |
| ----------- | -------------------------------------- | ----------- | ------------------ |
| main.rs:744 | handlers::member::router               | YES         | YES (member.rs:82) |
| main.rs:751 | handlers::courses::member_router       | NO          | NO                 |
| main.rs:752 | handlers::notifications::member_router | NO          | NO                 |

Routes hosted under the second/third nest (R-0260..R-0263 for courses, R-0316..R-0317 for notification-preferences) bypass both protective layers, despite all three nests sharing the same `/api/member` URL prefix.

---

## Mount: `/api/webhooks` (main.rs:754)

```
1. middleware::rate_limit::webhooks_layer   (webhooks.rs:42)
```

R-0318..R-0319.

## Mount: `/api` (csp-report — main.rs:756)

`csp_report::router` applies `clamp_body_size` (csp_report.rs:91) as a stand-alone middleware. R-0320.

## Mount: `/u` (main.rs:759) — public

No layer. R-0321.

## Mount: `/health, /ready, /version` (main.rs:710)

No layer. R-0322..R-0324.

## Mount: `/metrics` (main.rs:775)

In production: gated by `observability::handler::admin_metrics_handler` which itself runs the `AdminUser` extractor.
In dev: open public handler.

R-0325 — only route in this mount.

---

## SvelteKit surface

`/admin/**` SvelteKit routes are CSR-only (`+layout.ts: prerender=false, ssr=false`). No server-side load fns or form actions are present under `src/routes/admin/**`. The only SvelteKit server endpoint outside auth is `/api/greeks-pdf` (`src/routes/api/greeks-pdf/+server.ts`).

R-0326 (admin CSR marker), R-0327 (greeks-pdf).
