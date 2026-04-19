//! FDN-02: OpenAPI aggregation.
//!
//! Collects `#[utoipa::path(...)]` annotations on every mutating handler into a single
//! OpenAPI 3.1 document. The resulting JSON is served at `/api/openapi.json` (public in
//! non-production, admin-gated in production), and SwaggerUI is mounted at `/api/docs`.
//! The committed snapshot in `tests/snapshots/openapi.json` is the source of truth for
//! the frontend codegen (`scripts/openapi-to-ts.mjs`).

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    extractors::AdminUser,
    handlers::{
        admin, admin_dsar, admin_impersonation, admin_ip_allowlist, admin_members,
        admin_orders, admin_roles, admin_security, admin_settings, admin_subscriptions,
        analytics, auth, blog, consent, coupons, courses, csp_report, forms, member,
        notifications, outbox, popups, products, webhooks,
    },
    AppState,
};

// Note on `pricing.rs`: admin plan mutators are annotated in-module and are included
// via the `components(schemas(...))` aggregation below. The path `fn` references
// listed in `paths(...)` are the functions the macro expands path metadata onto.

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Swings API",
        description = "Swings public + admin HTTP API. Mutating endpoints are annotated; GETs are still fully callable but not listed here (see FDN-02 scope).",
        version = env!("CARGO_PKG_VERSION")
    ),
    servers(
        (url = "/", description = "Relative to deploying host")
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication and password flows"),
        (name = "admin", description = "Admin-only dashboard and member management"),
        (name = "admin-blog", description = "Admin-only blog management"),
        (name = "admin-notifications", description = "Admin-only notification template + delivery ops"),
        (name = "analytics", description = "Client-side analytics ingestion"),
        (name = "blog", description = "Public blog endpoints"),
        (name = "consent", description = "Cookie / tracking consent banner + category lookup"),
        (name = "coupons", description = "Discount coupon management"),
        (name = "courses", description = "Course catalog, modules, lessons, progress"),
        (name = "forms", description = "Form schema + submission endpoints"),
        (name = "member", description = "Authenticated member self-service"),
        (name = "popups", description = "Popup campaigns + submissions"),
        (name = "pricing", description = "Subscription pricing plans"),
        (name = "products", description = "EC-01 digital-goods product catalogue (admin + public)"),
        (name = "security", description = "Security telemetry (CSP violation reports, etc.)"),
        (name = "admin-security", description = "Admin-only privileged member-lifecycle and security console (suspend, ban, sessions, audit log)"),
        (name = "admin-impersonation", description = "Admin-only impersonation token mint / list / revoke"),
        (name = "admin-settings", description = "Admin-only typed settings catalogue (incl. maintenance-mode kill-switch)"),
        (name = "admin-roles", description = "Admin-only role / permission matrix CRUD with hot policy reload"),
        (name = "admin-members", description = "Admin-only members search + manual create"),
        (name = "admin-subscriptions", description = "Admin-only manual subscription operations (comp, extend, billing-cycle override)"),
        (name = "admin-orders", description = "Admin-only orders surface (list, manual create, void, partial refund, CSV export)"),
        (name = "admin-dsar", description = "Admin-initiated DSAR jobs (export + dual-control right-to-erasure tombstone)"),
        (name = "webhooks", description = "Inbound provider webhooks")
    ),
    paths(
        // Auth
        auth::register,
        auth::login,
        auth::refresh,
        auth::logout,
        auth::forgot_password,
        auth::reset_password,
        // Analytics
        analytics::ingest_events,
        // Admin
        admin::admin_member_billing_portal,
        admin::admin_member_subscription_cancel,
        admin::admin_member_subscription_resume,
        admin::update_member_role,
        admin::delete_member,
        admin::create_watchlist,
        admin::update_watchlist,
        admin::delete_watchlist,
        admin::create_alert,
        admin::update_alert,
        admin::delete_alert,
        // Admin security console (ADM-05)
        admin_security::suspend_member,
        admin_security::reactivate_member,
        admin_security::ban_member,
        admin_security::force_password_reset,
        admin_security::mark_email_verified,
        admin_security::list_sessions,
        admin_security::force_logout,
        admin_security::revoke_session,
        admin_security::list_audit_log,
        admin_security::list_failed_logins,
        // Admin IP allowlist (ADM-06)
        admin_ip_allowlist::list_entries,
        admin_ip_allowlist::create_entry,
        admin_ip_allowlist::delete_entry,
        admin_ip_allowlist::toggle_entry,
        // Admin impersonation (ADM-07)
        admin_impersonation::mint,
        admin_impersonation::list,
        admin_impersonation::get_one,
        admin_impersonation::revoke,
        admin_impersonation::exit,
        // Admin settings (ADM-08)
        admin_settings::list,
        admin_settings::get_one,
        admin_settings::upsert,
        admin_settings::reload,
        // Admin role matrix (ADM-09)
        admin_roles::list_matrix,
        admin_roles::list_permissions,
        admin_roles::grant,
        admin_roles::revoke,
        admin_roles::replace_role_permissions,
        admin_roles::reload,
        // Admin members (ADM-10)
        admin_members::search,
        admin_members::create,
        // Admin subscriptions (ADM-11)
        admin_subscriptions::by_user,
        admin_subscriptions::comp_grant,
        admin_subscriptions::extend_period,
        admin_subscriptions::override_billing_cycle,
        // Admin orders (ADM-12)
        admin_orders::list,
        admin_orders::read_one,
        admin_orders::create_manual,
        admin_orders::void_order,
        admin_orders::refund_order,
        admin_orders::export_csv,
        // Admin DSAR (ADM-13)
        admin_dsar::list_jobs,
        admin_dsar::read_job,
        admin_dsar::create_export,
        admin_dsar::request_erase,
        admin_dsar::approve_erase,
        admin_dsar::cancel_job,
        // Blog
        blog::admin_create_post,
        blog::admin_update_post,
        blog::admin_delete_post,
        blog::admin_restore_post_from_trash,
        blog::admin_update_post_status,
        blog::admin_autosave_post,
        blog::admin_restore_revision,
        blog::admin_create_category,
        blog::admin_update_category,
        blog::admin_delete_category,
        blog::admin_create_tag,
        blog::admin_delete_tag,
        blog::admin_upload_media,
        blog::admin_update_media,
        blog::admin_delete_media,
        blog::admin_upsert_post_meta,
        blog::admin_delete_post_meta,
        blog::public_unlock_post,
        // Courses
        courses::create_course,
        courses::update_course,
        courses::delete_course,
        courses::toggle_publish,
        courses::create_module,
        courses::update_module,
        courses::delete_module,
        courses::create_lesson,
        courses::update_lesson,
        courses::delete_lesson,
        courses::enroll_course,
        courses::update_lesson_progress,
        // Consent (CONSENT-01)
        consent::get_banner,
        // Consent event log + DSAR (CONSENT-03)
        consent::post_record,
        consent::get_my_consent,
        consent::post_dsar,
        consent::admin_list_dsar,
        consent::admin_fulfill_dsar,
        // Coupons
        coupons::admin_create_coupon,
        coupons::admin_update_coupon,
        coupons::admin_delete_coupon,
        coupons::admin_toggle_coupon,
        coupons::admin_bulk_create_coupons,
        coupons::admin_update_coupon_engine,
        coupons::public_validate_coupon,
        coupons::public_apply_coupon,
        // Member
        member::update_profile,
        member::post_billing_portal,
        member::post_subscription_cancel,
        member::post_subscription_resume,
        member::update_progress,
        // Outbox (FDN-04 admin ops)
        outbox::list_outbox,
        outbox::get_outbox,
        outbox::retry_outbox,
        // Notifications (FDN-05 admin + member)
        notifications::list_templates,
        notifications::create_template,
        notifications::get_template,
        notifications::update_template,
        notifications::preview_template,
        notifications::test_send_template,
        notifications::list_deliveries,
        notifications::list_suppression,
        notifications::add_suppression,
        notifications::remove_suppression,
        notifications::get_member_preferences,
        notifications::update_member_preferences,
        // Forms (FORM-03 .. FORM-10)
        forms::public_get_form,
        forms::public_submit,
        forms::public_save_partial,
        forms::admin_bulk_update_submissions,
        forms::public_create_payment_intent,
        forms::public_geo_countries,
        forms::public_geo_states,
        // Popups
        popups::admin_create_popup,
        popups::admin_update_popup,
        popups::admin_delete_popup,
        popups::admin_toggle_popup,
        popups::admin_duplicate_popup,
        popups::public_track_event,
        popups::public_submit_form,
        // Pricing
        crate::handlers::pricing::admin_create_plan,
        crate::handlers::pricing::admin_update_plan,
        crate::handlers::pricing::admin_delete_plan,
        crate::handlers::pricing::admin_toggle_plan,
        // Products (EC-01)
        products::admin_create_product,
        products::admin_update_product,
        products::admin_delete_product,
        products::admin_set_status,
        products::admin_add_variant,
        products::admin_update_variant,
        products::admin_delete_variant,
        products::admin_add_asset,
        products::admin_delete_asset,
        products::admin_set_bundle_items,
        // Webhooks
        webhooks::stripe_webhook,
        webhooks::resend_email_webhook,
        // Security (FDN-08)
        csp_report::csp_report,
    ),
    components(
        schemas(
            // Auth + user
            crate::models::User,
            crate::models::UserRole,
            crate::models::UserResponse,
            crate::models::RegisterRequest,
            crate::models::LoginRequest,
            crate::models::AuthResponse,
            crate::models::RefreshRequest,
            crate::models::TokenResponse,
            crate::models::ForgotPasswordRequest,
            crate::models::ResetPasswordRequest,
            // Subscription
            crate::models::Subscription,
            crate::models::SubscriptionPlan,
            crate::models::SubscriptionStatus,
            crate::models::SubscriptionStatusResponse,
            crate::models::BillingPortalResponse,
            crate::models::BillingPortalRequest,
            // Watchlist
            crate::models::Watchlist,
            crate::models::WatchlistAlert,
            crate::models::WatchlistWithAlerts,
            crate::models::TradeDirection,
            crate::models::CreateWatchlistRequest,
            crate::models::UpdateWatchlistRequest,
            crate::models::CreateAlertRequest,
            crate::models::UpdateAlertRequest,
            // Admin
            crate::models::AdminStats,
            // Admin security console (ADM-05)
            admin_security::LifecycleRequest,
            admin_security::ForcePasswordResetResponse,
            admin_security::SessionRow,
            admin_security::SessionsResponse,
            admin_security::AuditLogRow,
            admin_security::AuditLogFilter,
            admin_security::AuditLogResponse,
            admin_security::FailedLoginRow,
            admin_security::FailedLoginFilter,
            admin_security::FailedLoginResponse,
            // Admin IP allowlist (ADM-06)
            crate::security::ip_allowlist::AllowlistEntry,
            crate::security::ip_allowlist::CreateAllowlistInput,
            admin_ip_allowlist::AllowlistResponse,
            admin_ip_allowlist::ToggleRequest,
            crate::security::impersonation::ImpersonationSession,
            crate::security::impersonation::CreateImpersonationInput,
            admin_impersonation::MintResponse,
            admin_impersonation::ListResponse,
            admin_impersonation::RevokeRequest,
            // Admin settings (ADM-08)
            crate::settings::SettingType,
            crate::settings::SettingRecord,
            crate::settings::SettingView,
            admin_settings::SettingListResponse,
            admin_settings::SettingGetResponse,
            admin_settings::SettingUpsertRequest,
            // Admin role matrix (ADM-09)
            admin_roles::PermissionRow,
            admin_roles::PermissionsResponse,
            admin_roles::RolePermPair,
            admin_roles::MatrixResponse,
            admin_roles::ReplaceRoleRequest,
            // Admin members (ADM-10)
            admin_members::CreateMemberRequest,
            admin_members::CreateMemberResponse,
            // Admin subscriptions (ADM-11)
            admin_subscriptions::CompGrantRequest,
            admin_subscriptions::CompGrantResponse,
            admin_subscriptions::ExtendRequest,
            admin_subscriptions::ExtendResponse,
            admin_subscriptions::CycleOverrideRequest,
            admin_subscriptions::CycleOverrideResponse,
            admin_subscriptions::MembershipRow,
            admin_subscriptions::UserSubscriptionView,
            // Admin orders (ADM-12)
            admin_orders::OrderListEnvelope,
            admin_orders::OrderDetail,
            admin_orders::ManualOrderItem,
            admin_orders::ManualOrderRequest,
            admin_orders::VoidRequest,
            admin_orders::RefundRequest,
            admin_orders::RefundResponse,
            // Admin DSAR (ADM-13)
            admin_dsar::DsarJob,
            admin_dsar::JobListEnvelope,
            admin_dsar::ExportRequest,
            admin_dsar::ExportResponse,
            admin_dsar::EraseRequestBody,
            admin_dsar::EraseApproveBody,
            admin_dsar::EraseApproveResponse,
            admin_dsar::CancelBody,
            crate::services::dsar_admin::TombstoneSummary,
            // Blog
            crate::models::BlogPost,
            crate::models::BlogPostResponse,
            crate::models::BlogPostListItem,
            crate::models::PostStatus,
            crate::models::CreatePostRequest,
            crate::models::UpdatePostRequest,
            crate::models::UpdatePostStatusRequest,
            crate::models::VerifyPostPasswordRequest,
            crate::models::PostMeta,
            crate::models::UpsertPostMetaRequest,
            crate::models::BlogCategory,
            crate::models::CreateCategoryRequest,
            crate::models::UpdateCategoryRequest,
            crate::models::BlogTag,
            crate::models::CreateTagRequest,
            crate::models::BlogRevision,
            crate::models::RevisionResponse,
            crate::models::Media,
            crate::models::UpdateMediaRequest,
            // Courses
            crate::models::Course,
            crate::models::CourseModule,
            crate::models::CourseLesson,
            crate::models::LessonProgress,
            crate::models::CreateCourseRequest,
            crate::models::UpdateCourseRequest,
            crate::models::CreateModuleRequest,
            crate::models::UpdateModuleRequest,
            crate::models::CreateLessonRequest,
            crate::models::UpdateLessonRequest,
            crate::models::UpdateLessonProgressRequest,
            crate::models::CourseWithModules,
            crate::models::ModuleWithLessons,
            crate::models::CourseListItem,
            crate::models::CourseEnrollment,
            // Pricing
            crate::models::PricingPlan,
            crate::models::CreatePricingPlanRequest,
            crate::models::UpdatePricingPlanRequest,
            // Coupons
            crate::models::DiscountType,
            crate::models::Coupon,
            crate::models::CreateCouponRequest,
            crate::models::UpdateCouponRequest,
            crate::models::ValidateCouponRequest,
            crate::models::CouponValidationResponse,
            crate::models::BulkCouponRequest,
            // EC-11 coupon-engine DTOs
            coupons::UpdateCouponEngineRequest,
            coupons::CouponEngineView,
            crate::commerce::coupons::CouponScope,
            crate::commerce::coupons::RecurringMode,
            crate::commerce::coupons::BogoConfig,
            // Consent (CONSENT-01)
            consent::BannerConfig,
            consent::BannerCopy,
            consent::BannerLayout,
            consent::BannerPosition,
            consent::ConsentCategoryDef,
            // Products (EC-01)
            crate::commerce::products::Product,
            crate::commerce::products::ProductVariant,
            crate::commerce::products::DownloadableAsset,
            crate::commerce::products::BundleItem,
            crate::commerce::products::BundleItemInput,
            crate::commerce::products::ProductType,
            crate::commerce::products::ProductStatus,
            crate::commerce::products::CreateProductRequest,
            crate::commerce::products::UpdateProductRequest,
            crate::commerce::products::SetStatusRequest,
            crate::commerce::products::CreateVariantRequest,
            crate::commerce::products::UpdateVariantRequest,
            crate::commerce::products::CreateAssetRequest,
            crate::commerce::products::SetBundleItemsRequest,
            products::ProductDetail,
            // Forms (FORM-01..10)
            forms::FormDefinition,
            forms::SubmitRequest,
            forms::SubmitResponse,
            forms::PartialRequest,
            forms::PartialResponse,
            forms::PartialLoadResponse,
            forms::PaginatedSubmissions,
            forms::BulkActionRequest,
            forms::BulkActionResponse,
            forms::PaymentIntentRequest,
            forms::PaymentIntentClientResponse,
            crate::forms::repo::SubmissionRow,
            crate::forms::repo::FormRow,
            crate::forms::repo::FormVersionRow,
            crate::forms::repo::PartialRow,
            crate::forms::validation::ValidationError,
            crate::forms::geo::Country,
            crate::forms::geo::State,
            // Popups
            crate::models::Popup,
            crate::models::CreatePopupRequest,
            crate::models::UpdatePopupRequest,
            crate::models::PopupSubmission,
            crate::models::PopupSubmitRequest,
            crate::models::PopupAnalytics,
            popups::TrackEventRequest,
            // Pagination + analytics
            crate::models::PaginationParams,
            crate::models::AnalyticsIngestRequest,
            crate::models::AnalyticsIngestEvent,
            // Handler-local
            admin::RoleUpdate,
            member::UpdateProfileRequest,
            // Outbox DTOs (FDN-04)
            outbox::OutboxRowDto,
            outbox::OutboxRetryResponse,
            outbox::PaginatedOutboxResponse,
            crate::events::outbox::OutboxStatus,
            // Notifications (FDN-05)
            notifications::TemplateListQuery,
            notifications::CreateTemplateRequest,
            notifications::UpdateTemplateRequest,
            notifications::PreviewRequest,
            notifications::TestSendRequest,
            notifications::TestSendResponse,
            notifications::DeliveryListQuery,
            notifications::DeliveryRow,
            notifications::PaginatedDeliveriesResponse,
            notifications::SuppressionListQuery,
            notifications::PaginatedSuppressionResponse,
            notifications::AddSuppressionRequest,
            notifications::RemoveSuppressionRequest,
            notifications::BulkPreferenceUpdate,
            notifications::MemberPreferencesResponse,
            notifications::PaginatedTemplatesResponse,
            crate::notifications::templates::Template,
            crate::notifications::templates::RenderedTemplate,
            crate::notifications::preferences::NotificationPreference,
            crate::notifications::preferences::PreferenceUpdate,
            crate::notifications::suppression::Suppression,
        )
    )
)]
pub struct ApiDoc;

/// Mount `/api/openapi.json` (gated in production) and SwaggerUI at `/api/docs`.
pub fn mount(app: Router<AppState>, state: &AppState) -> Router<AppState> {
    let is_prod = state.config.is_production();
    // SwaggerUI is always mounted; the JSON handler enforces prod admin gating.
    let swagger = SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi());
    if is_prod {
        app.route("/api/openapi.json", get(protected_openapi_json))
            .merge(swagger)
    } else {
        app.route("/api/openapi.json", get(public_openapi_json))
            .merge(swagger)
    }
}

async fn public_openapi_json(State(_state): State<AppState>) -> Response {
    Json(ApiDoc::openapi()).into_response()
}

/// Admin-gated variant used in production. `AdminUser` returns 401/403 via `AppError`.
async fn protected_openapi_json(
    State(_state): State<AppState>,
    _admin: AdminUser,
) -> Result<Response, StatusCode> {
    Ok(Json(ApiDoc::openapi()).into_response())
}
