# DB Touchpoints — Phase 1 Surface Map

Per-table inventory of writes / reads / locks observed across `backend/src/handlers/`, `backend/src/commerce/`, `backend/src/services/`, `backend/src/security/`, `backend/src/notifications/`, `backend/src/popups/`, `backend/src/forms/`, `backend/src/consent/`, `backend/src/settings/`, `backend/src/events/`, and `backend/src/db.rs`.

Counts of write SQL statements grepped by table:

| Table                          | Writes occur in (files) |
|--------------------------------|-------------------------|
| `users`                        | db.rs, handlers/admin.rs, handlers/admin_security.rs, handlers/auth.rs, handlers/member.rs, services/dsar_admin.rs |
| `subscriptions`                | commerce/billing.rs, commerce/subscriptions.rs, handlers/admin.rs, handlers/admin_subscriptions.rs, handlers/member.rs, handlers/webhooks.rs |
| `memberships`                  | commerce/billing.rs, commerce/orders.rs, db.rs, handlers/admin_subscriptions.rs |
| `orders`                       | commerce/orders.rs, handlers/admin_orders.rs, handlers/webhooks.rs |
| `order_items`                  | commerce/orders.rs, handlers/admin_orders.rs |
| `order_state_transitions`      | commerce/orders.rs, handlers/admin_orders.rs |
| `order_refunds`                | commerce/refunds.rs, handlers/admin_orders.rs |
| `order_notes`                  | handlers/admin_orders.rs |
| `subscription_invoices`        | commerce/billing.rs, handlers/webhooks.rs |
| `subscription_changes`         | handlers/member.rs, handlers/webhooks.rs |
| `subscription_trial_events`    | handlers/webhooks.rs |
| `payment_disputes`             | commerce/disputes.rs |
| `payment_failures`             | handlers/webhooks.rs |
| `payment_refunds`              | commerce/refunds.rs, handlers/webhooks.rs |
| `dunning_attempts`             | handlers/webhooks.rs |
| `processed_webhook_events`     | handlers/webhooks.rs |
| `stripe_webhook_audit`         | commerce/webhook_audit.rs |
| `refresh_tokens`               | db.rs, handlers/auth.rs, handlers/admin_security.rs |
| `password_reset_tokens`        | db.rs, handlers/auth.rs, handlers/admin_security.rs |
| `email_verification_tokens`    | db.rs, handlers/auth.rs |
| `failed_login_attempts`        | handlers/auth.rs |
| `impersonation_sessions`       | security/impersonation.rs, handlers/admin_impersonation.rs |
| `admin_actions`                | services/audit.rs (every audit_admin / audit_admin_priv / record_admin_action) |
| `admin_ip_allowlist`           | handlers/admin_ip_allowlist.rs, security/ip_allowlist.rs |
| `app_settings`                 | settings/cache.rs, handlers/admin_settings.rs |
| `permissions`                  | (read-only path; populated by migrations 021 / 0NN_*_perms) |
| `role_permissions`             | handlers/admin_roles.rs, authz.rs (load) |
| `idempotency_keys`             | middleware/idempotency.rs, services/idempotency_gc.rs |
| `outbox_events`                | events/outbox.rs, handlers/outbox.rs (retry) |
| `dsar_jobs`                    | handlers/admin_dsar.rs, services/dsar_worker.rs, services/dsar_admin.rs, services/dsar_artifact_sweep.rs |
| `dsar_requests`                | handlers/consent.rs |
| `consent_records`              | consent/repo.rs, handlers/consent.rs |
| `consent_integrity_anchors`    | consent/integrity.rs |
| `consent_banner_configs`       | handlers/admin_consent.rs |
| `consent_categories`           | handlers/admin_consent.rs |
| `consent_services`             | handlers/admin_consent.rs |
| `consent_policies`             | handlers/admin_consent.rs |
| `notification_templates`       | handlers/notifications.rs |
| `notification_deliveries`      | handlers/notifications.rs, notifications/channels/*.rs, handlers/webhooks.rs |
| `notification_suppression`     | handlers/notifications.rs, notifications/suppression.rs, handlers/webhooks.rs |
| `notification_preferences`     | notifications/preferences.rs, handlers/notifications.rs |
| `unsubscribe_tokens`           | notifications/unsubscribe.rs, handlers/notifications.rs |
| `blog_posts`                   | handlers/blog.rs |
| `blog_revisions`               | handlers/blog.rs, services/blog_scheduler.rs |
| `blog_categories`              | handlers/blog.rs |
| `blog_post_categories`         | handlers/blog.rs |
| `blog_tags`                    | handlers/blog.rs |
| `blog_post_tags`               | handlers/blog.rs |
| `post_meta`                    | handlers/blog.rs |
| `media`                        | handlers/blog.rs |
| `courses`                      | handlers/courses.rs |
| `course_modules`               | handlers/courses.rs |
| `course_lessons`               | handlers/courses.rs |
| `course_enrollments`           | handlers/courses.rs, handlers/member.rs |
| `lesson_progress`              | handlers/courses.rs, handlers/member.rs |
| `coupons`                      | handlers/coupons.rs |
| `coupon_usages`                | handlers/coupons.rs, handlers/member.rs |
| `pricing_plans`                | handlers/pricing.rs, services/pricing_rollout.rs |
| `pricing_change_log`           | handlers/pricing.rs |
| `membership_plans`             | (read paths only) |
| `popups`                       | handlers/popups.rs |
| `popup_variants`               | handlers/popups.rs |
| `popup_events`                 | handlers/popups.rs |
| `popup_submissions`            | handlers/popups.rs |
| `popup_visitor_state`          | handlers/popups.rs |
| `popup_attributions`           | events/handlers/popup_attribution.rs |
| `products`                     | commerce/repo.rs, handlers/products.rs |
| `product_variants`             | commerce/repo.rs, handlers/products.rs |
| `bundle_items`                 | commerce/repo.rs, handlers/products.rs |
| `downloadable_assets`          | commerce/repo.rs, handlers/products.rs |
| `user_downloads`               | events/handlers/digital_delivery.rs |
| `forms`                        | forms/repo.rs |
| `form_versions`                | forms/repo.rs |
| `form_submissions`             | forms/repo.rs, handlers/forms.rs |
| `form_partials`                | forms/repo.rs, handlers/forms.rs |
| `form_uploads`                 | handlers/forms.rs |
| `form_payment_intents`         | handlers/forms.rs |
| `analytics_events`             | handlers/analytics.rs |
| `analytics_sessions`           | handlers/analytics.rs |
| `cart`/`carts`/`cart_items`    | handlers/cart.rs |
| `addresses`                    | commerce/orders.rs |
| `watchlists`                   | handlers/admin.rs |
| `watchlist_alerts`             | handlers/admin.rs |

---

## Locks observed (`FOR UPDATE` / `FOR SHARE` / `SKIP LOCKED`)

Source-line evidence (machine-checkable):

| Location                                            | Lock variant                |
|-----------------------------------------------------|-----------------------------|
| `commerce/orders.rs:289`                            | `FOR UPDATE` on `orders`    |
| `events/outbox.rs:185`                              | `FOR UPDATE SKIP LOCKED`    |
| `handlers/admin_subscriptions.rs:324`               | `FOR UPDATE` on `subscriptions` (extend_period) |
| `handlers/admin_subscriptions.rs:416`               | `FOR UPDATE` on `subscriptions` (override_billing_cycle) |
| `handlers/admin_roles.rs:374`                       | `FOR UPDATE` on `role_permissions` (replace_role_permissions) |
| `handlers/admin_orders.rs:559`                      | `FOR UPDATE` on `orders` (refund_order) |
| `notifications/unsubscribe.rs:132`                  | `FOR UPDATE` on `unsubscribe_tokens` |
| `services/dsar_admin.rs:130`                        | `FOR UPDATE` on `users` (erase) |
| `services/dsar_artifact_sweep.rs:154`               | `FOR UPDATE SKIP LOCKED` on `dsar_jobs` |
| `services/dsar_worker.rs:131`                       | `FOR UPDATE SKIP LOCKED` on `dsar_jobs` |
| `services/idempotency_gc.rs:73`                     | `FOR UPDATE SKIP LOCKED` on `idempotency_keys` |
| `services/blog_scheduler.rs:29`                     | (comment-only — claim is monotonic; no per-row lock) |

No explicit `SET TRANSACTION ISOLATION LEVEL` overrides observed in the searched tree (default `READ COMMITTED` applies).

---

## Transactions (`PgPool::begin`)

Files that open explicit transactions in the admin-mutation surface (machine-checkable list of `.begin()` call sites — full match list in `/tmp/tx_locks.txt`):

```
backend/src/commerce/orders.rs           — checkout / order state-machine
backend/src/commerce/billing.rs          — invoice + subscription writes
backend/src/commerce/refunds.rs          — refund + audit
backend/src/commerce/subscriptions.rs    — Stripe-driven status updates
backend/src/handlers/admin_subscriptions.rs (comp_grant/extend_period/override_billing_cycle)
backend/src/handlers/admin_orders.rs     (create_manual / void / refund)
backend/src/handlers/admin_dsar.rs       (request_erase / approve_erase / cancel)
backend/src/handlers/admin_roles.rs      (replace_role_permissions)
backend/src/handlers/admin_impersonation.rs (mint / revoke)
backend/src/handlers/blog.rs             (create_post / restore_revision)
backend/src/handlers/courses.rs          (delete_course cascade)
backend/src/handlers/popups.rs           (duplicate_popup)
backend/src/handlers/products.rs         (admin_set_bundle_items)
backend/src/handlers/coupons.rs          (admin_bulk_create_coupons)
backend/src/handlers/forms.rs            (public_submit — submission + outbox)
backend/src/handlers/webhooks.rs         (every Stripe handle_* fn)
backend/src/handlers/auth.rs             (refresh-token rotation)
backend/src/middleware/idempotency.rs    (claim row + handler dispatch)
backend/src/services/dsar_admin.rs       (erase tombstone)
backend/src/services/dsar_worker.rs      (compose + upload)
backend/src/services/dsar_artifact_sweep.rs
backend/src/notifications/unsubscribe.rs (token consume)
backend/src/events/outbox.rs             (publish_in_tx + claim/finalize)
```

---

## Notes for downstream phases

- The `audit_admin*` family always inserts into `admin_actions`. Several handlers call it AFTER the primary write but in the SAME transaction; others use `record_admin_action_best_effort` which deliberately runs outside the tx (so a failed audit cannot rollback the user-visible mutation). This split is preserved in the per-route `audit_record_calls` field.
- `update_member_role` (R-0012, admin.rs:624) writes `users` then `admin_actions` via `record_admin_action_best_effort` — the audit row is not transactional with the role update. Same pattern in R-0013 (delete_member) and R-0006 (update_member_profile).
- Webhook handlers on `/api/webhooks/stripe` write into 13+ tables across the request — all wrapped in a single tx that also writes the dedup row (`processed_webhook_events`), per webhooks.rs:56.
- The outbox dispatcher uses `SELECT ... FOR UPDATE SKIP LOCKED` so multiple workers do not collide. No admin handler writes `outbox_events` directly; they use `events::publish_in_tx` to enqueue inside their own tx.
