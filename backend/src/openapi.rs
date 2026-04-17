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
        admin, analytics, auth, blog, consent, coupons, courses, csp_report, member, notifications,
        outbox, popups, webhooks,
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
        (name = "member", description = "Authenticated member self-service"),
        (name = "popups", description = "Popup campaigns + submissions"),
        (name = "pricing", description = "Subscription pricing plans"),
        (name = "security", description = "Security telemetry (CSP violation reports, etc.)"),
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
            // Consent (CONSENT-01)
            consent::BannerConfig,
            consent::BannerCopy,
            consent::BannerLayout,
            consent::BannerPosition,
            consent::ConsentCategoryDef,
            // Consent event log + DSAR (CONSENT-03)
            consent::ConsentRecordRequest,
            consent::ConsentRecordResponse,
            consent::MyConsentResponse,
            consent::DsarSubmitRequest,
            consent::DsarSubmitResponse,
            consent::DsarListResponse,
            consent::DsarFulfillRequest,
            consent::DsarFulfillResponse,
            crate::consent::records::ConsentRecordRow,
            crate::consent::records::DsarRow,
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
