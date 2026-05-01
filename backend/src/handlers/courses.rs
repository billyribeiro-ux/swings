use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{AdminUser, AuthUser, ClientInfo, MaybeAuthUser},
    models::*,
    services::audit::audit_admin,
    AppState,
};

// ── Routers ──────────────────────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_list_courses).post(create_course))
        .route(
            "/{id}",
            get(admin_get_course)
                .put(update_course)
                .delete(delete_course),
        )
        .route("/{id}/publish", post(toggle_publish))
        .route("/{id}/modules", post(create_module))
        .route(
            "/{course_id}/modules/{module_id}",
            put(update_module).delete(delete_module),
        )
        .route(
            "/{course_id}/modules/{module_id}/lessons",
            post(create_lesson),
        )
        .route(
            "/lessons/{lesson_id}",
            put(update_lesson).delete(delete_lesson),
        )
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(public_list_courses))
        .route("/{slug}", get(public_get_course))
}

pub fn member_router() -> Router<AppState> {
    Router::new()
        .route("/courses/{course_id}/enroll", post(enroll_course))
        .route("/courses/{course_id}/progress", get(get_course_progress))
        .route("/lessons/{lesson_id}/progress", put(update_lesson_progress))
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── Admin Handlers ───────────────────────────────────────────────────────

async fn admin_list_courses(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<CourseListItem>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM courses")
        .fetch_one(&state.db)
        .await?;

    let courses = sqlx::query_as::<_, CourseListItem>(
        r#"
        SELECT c.id, c.title, c.slug, c.short_description, c.thumbnail_url,
               c.difficulty, u.name AS instructor_name, c.price_cents,
               c.is_free, c.is_included_in_subscription, c.published,
               c.estimated_duration_minutes,
               COUNT(cl.id)::bigint AS total_lessons,
               c.created_at
        FROM courses c
        JOIN users u ON u.id = c.instructor_id
        LEFT JOIN course_modules cm ON cm.course_id = c.id
        LEFT JOIN course_lessons cl ON cl.module_id = cm.id
        GROUP BY c.id, u.name
        ORDER BY c.sort_order ASC, c.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total_pages = (total + per_page - 1) / per_page;

    Ok(Json(PaginatedResponse {
        data: courses,
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/courses",
    tag = "courses",
    security(("bearer_auth" = [])),
    request_body = CreateCourseRequest,
    responses(
        (status = 200, description = "Course created", body = Course),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn create_course(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(mut req): Json<CreateCourseRequest>,
) -> AppResult<Json<Course>> {
    admin.require(&state.policy, "course.manage")?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // SECURITY (XSS): sanitize user-supplied course copy before we persist it.
    if let Some(d) = req.description.as_deref() {
        req.description = Some(crate::common::html::sanitize_rich_text(d));
    }
    if let Some(s) = req.short_description.as_deref() {
        req.short_description = Some(crate::common::html::sanitize_plain_text(s));
    }

    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.title));
    let description = req.description.as_deref().unwrap_or("");
    let difficulty = req.difficulty.as_deref().unwrap_or("beginner");
    let price_cents = req.price_cents.unwrap_or(0);
    let currency = req.currency.as_deref().unwrap_or("usd");
    let is_free = req.is_free.unwrap_or(false);
    let is_included_in_subscription = req.is_included_in_subscription.unwrap_or(false);
    let sort_order = req.sort_order.unwrap_or(0);
    let published = req.published.unwrap_or(false);
    let estimated_duration_minutes = req.estimated_duration_minutes.unwrap_or(0);
    let published_at: Option<chrono::DateTime<Utc>> =
        if published { Some(Utc::now()) } else { None };

    let course = sqlx::query_as::<_, Course>(
        r#"
        INSERT INTO courses
            (title, slug, description, short_description, thumbnail_url,
             trailer_video_url, difficulty, instructor_id, price_cents, currency,
             is_free, is_included_in_subscription, sort_order, published,
             published_at, estimated_duration_minutes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING id, title, slug, description, short_description, thumbnail_url,
                  trailer_video_url, difficulty, instructor_id, price_cents, currency,
                  is_free, is_included_in_subscription, sort_order, published,
                  published_at, estimated_duration_minutes, created_at, updated_at
        "#,
    )
    .bind(&req.title)
    .bind(&slug)
    .bind(description)
    .bind(&req.short_description)
    .bind(&req.thumbnail_url)
    .bind(&req.trailer_video_url)
    .bind(difficulty)
    .bind(admin.user_id)
    .bind(price_cents)
    .bind(currency)
    .bind(is_free)
    .bind(is_included_in_subscription)
    .bind(sort_order)
    .bind(published)
    .bind(published_at)
    .bind(estimated_duration_minutes)
    .fetch_one(&state.db)
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.create",
        "course",
        course.id,
        serde_json::json!({
            "slug": course.slug,
            "published": course.published,
        }),
    )
    .await;

    Ok(Json(course))
}

async fn admin_get_course(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CourseWithModules>> {
    let course = sqlx::query_as::<_, Course>(
        r#"
        SELECT id, title, slug, description, short_description, thumbnail_url,
               trailer_video_url, difficulty, instructor_id, price_cents, currency,
               is_free, is_included_in_subscription, sort_order, published,
               published_at, estimated_duration_minutes, created_at, updated_at
        FROM courses
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let modules = sqlx::query_as::<_, CourseModule>(
        r#"
        SELECT id, course_id, title, description, sort_order, created_at, updated_at
        FROM course_modules
        WHERE course_id = $1
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let lessons = sqlx::query_as::<_, CourseLesson>(
        r#"
        SELECT cl.id, cl.module_id, cl.title, cl.slug, cl.description,
               cl.content, cl.content_json, cl.video_url, cl.video_duration_seconds,
               cl.sort_order, cl.is_preview, cl.created_at, cl.updated_at
        FROM course_lessons cl
        JOIN course_modules cm ON cm.id = cl.module_id
        WHERE cm.course_id = $1
        ORDER BY cl.sort_order ASC, cl.created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let total_lessons = lessons.len() as i64;
    let total_duration_seconds: i64 = lessons
        .iter()
        .filter_map(|l| l.video_duration_seconds)
        .map(|d| d as i64)
        .sum();

    let mut modules_with_lessons: Vec<ModuleWithLessons> = modules
        .into_iter()
        .map(|m| ModuleWithLessons {
            module: m,
            lessons: Vec::new(),
        })
        .collect();

    for lesson in lessons {
        if let Some(mwl) = modules_with_lessons
            .iter_mut()
            .find(|mwl| mwl.module.id == lesson.module_id)
        {
            mwl.lessons.push(lesson);
        }
    }

    Ok(Json(CourseWithModules {
        course,
        modules: modules_with_lessons,
        total_lessons,
        total_duration_seconds,
    }))
}

#[utoipa::path(
    put,
    path = "/api/admin/courses/{id}",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Course id")),
    request_body = UpdateCourseRequest,
    responses(
        (status = 200, description = "Course updated", body = Course),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Course not found")
    )
)]
pub(crate) async fn update_course(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(mut req): Json<UpdateCourseRequest>,
) -> AppResult<Json<Course>> {
    admin.require(&state.policy, "course.manage")?;
    // SECURITY (XSS): sanitize at the write boundary — see `create_course`.
    if let Some(d) = req.description.as_deref() {
        req.description = Some(crate::common::html::sanitize_rich_text(d));
    }
    if let Some(s) = req.short_description.as_deref() {
        req.short_description = Some(crate::common::html::sanitize_plain_text(s));
    }

    let _existing = sqlx::query_scalar::<_, Uuid>("SELECT id FROM courses WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let slug = req.slug.as_deref().map(slugify);

    let course = sqlx::query_as::<_, Course>(
        r#"
        UPDATE courses SET
            title = COALESCE($2, title),
            slug = COALESCE($3, slug),
            description = COALESCE($4, description),
            short_description = COALESCE($5, short_description),
            thumbnail_url = COALESCE($6, thumbnail_url),
            trailer_video_url = COALESCE($7, trailer_video_url),
            difficulty = COALESCE($8, difficulty),
            price_cents = COALESCE($9, price_cents),
            currency = COALESCE($10, currency),
            is_free = COALESCE($11, is_free),
            is_included_in_subscription = COALESCE($12, is_included_in_subscription),
            sort_order = COALESCE($13, sort_order),
            published = COALESCE($14, published),
            estimated_duration_minutes = COALESCE($15, estimated_duration_minutes),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, title, slug, description, short_description, thumbnail_url,
                  trailer_video_url, difficulty, instructor_id, price_cents, currency,
                  is_free, is_included_in_subscription, sort_order, published,
                  published_at, estimated_duration_minutes, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&req.title)
    .bind(&slug)
    .bind(&req.description)
    .bind(&req.short_description)
    .bind(&req.thumbnail_url)
    .bind(&req.trailer_video_url)
    .bind(&req.difficulty)
    .bind(req.price_cents)
    .bind(&req.currency)
    .bind(req.is_free)
    .bind(req.is_included_in_subscription)
    .bind(req.sort_order)
    .bind(req.published)
    .bind(req.estimated_duration_minutes)
    .fetch_one(&state.db)
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.update",
        "course",
        course.id,
        serde_json::json!({
            "slug": course.slug,
            "published": course.published,
        }),
    )
    .await;

    Ok(Json(course))
}

#[utoipa::path(
    delete,
    path = "/api/admin/courses/{id}",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Course id")),
    responses(
        (status = 200, description = "Course deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Course not found")
    )
)]
pub(crate) async fn delete_course(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "course.manage")?;
    let snapshot: Option<(String,)> = sqlx::query_as("SELECT slug FROM courses WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let slug = snapshot
        .ok_or(AppError::NotFound("Course not found".to_string()))?
        .0;

    let rows = sqlx::query("DELETE FROM courses WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Course not found".to_string()));
    }

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.delete",
        "course",
        id,
        serde_json::json!({ "slug": slug }),
    )
    .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[utoipa::path(
    post,
    path = "/api/admin/courses/{id}/publish",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Course id")),
    responses(
        (status = 200, description = "Publish toggled", body = Course),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Course not found")
    )
)]
pub(crate) async fn toggle_publish(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Course>> {
    admin.require(&state.policy, "course.manage")?;
    let course = sqlx::query_as::<_, Course>(
        r#"
        UPDATE courses SET
            published = NOT published,
            published_at = CASE
                WHEN NOT published THEN NOW()
                ELSE published_at
            END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, title, slug, description, short_description, thumbnail_url,
                  trailer_video_url, difficulty, instructor_id, price_cents, currency,
                  is_free, is_included_in_subscription, sort_order, published,
                  published_at, estimated_duration_minutes, created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".to_string()))?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.publish.toggle",
        "course",
        course.id,
        serde_json::json!({
            "slug": course.slug,
            "published": course.published,
        }),
    )
    .await;

    Ok(Json(course))
}

// ── Module Handlers ──────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/admin/courses/{id}/modules",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Course id")),
    request_body = CreateModuleRequest,
    responses(
        (status = 200, description = "Module created", body = CourseModule),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Course not found"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn create_module(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(course_id): Path<Uuid>,
    Json(req): Json<CreateModuleRequest>,
) -> AppResult<Json<CourseModule>> {
    admin.require(&state.policy, "course.manage")?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify course exists
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM courses WHERE id = $1")
        .bind(course_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let sort_order = req.sort_order.unwrap_or(0);

    let module = sqlx::query_as::<_, CourseModule>(
        r#"
        INSERT INTO course_modules (course_id, title, description, sort_order)
        VALUES ($1, $2, $3, $4)
        RETURNING id, course_id, title, description, sort_order, created_at, updated_at
        "#,
    )
    .bind(course_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(sort_order)
    .fetch_one(&state.db)
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.module.create",
        "course_module",
        module.id,
        serde_json::json!({
            "course_id": course_id,
            "title": module.title,
        }),
    )
    .await;

    Ok(Json(module))
}

#[utoipa::path(
    put,
    path = "/api/admin/courses/{course_id}/modules/{module_id}",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(
        ("course_id" = Uuid, Path, description = "Course id"),
        ("module_id" = Uuid, Path, description = "Module id")
    ),
    request_body = UpdateModuleRequest,
    responses(
        (status = 200, description = "Module updated", body = CourseModule),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Module not found")
    )
)]
pub(crate) async fn update_module(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path((course_id, module_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateModuleRequest>,
) -> AppResult<Json<CourseModule>> {
    admin.require(&state.policy, "course.manage")?;
    let module = sqlx::query_as::<_, CourseModule>(
        r#"
        UPDATE course_modules SET
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            sort_order = COALESCE($5, sort_order),
            updated_at = NOW()
        WHERE id = $2 AND course_id = $1
        RETURNING id, course_id, title, description, sort_order, created_at, updated_at
        "#,
    )
    .bind(course_id)
    .bind(module_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.sort_order)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Module not found".to_string()))?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.module.update",
        "course_module",
        module.id,
        serde_json::json!({
            "course_id": course_id,
            "title": module.title,
        }),
    )
    .await;

    Ok(Json(module))
}

#[utoipa::path(
    delete,
    path = "/api/admin/courses/{course_id}/modules/{module_id}",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(
        ("course_id" = Uuid, Path, description = "Course id"),
        ("module_id" = Uuid, Path, description = "Module id")
    ),
    responses(
        (status = 200, description = "Module deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Module not found")
    )
)]
pub(crate) async fn delete_module(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path((course_id, module_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "course.manage")?;
    let rows = sqlx::query("DELETE FROM course_modules WHERE id = $1 AND course_id = $2")
        .bind(module_id)
        .bind(course_id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Module not found".to_string()));
    }

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.module.delete",
        "course_module",
        module_id,
        serde_json::json!({ "course_id": course_id }),
    )
    .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Lesson Handlers ──────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/admin/courses/{course_id}/modules/{module_id}/lessons",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(
        ("course_id" = Uuid, Path, description = "Course id"),
        ("module_id" = Uuid, Path, description = "Module id")
    ),
    request_body = CreateLessonRequest,
    responses(
        (status = 200, description = "Lesson created", body = CourseLesson),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Module not found"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn create_lesson(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path((course_id, module_id)): Path<(Uuid, Uuid)>,
    Json(mut req): Json<CreateLessonRequest>,
) -> AppResult<Json<CourseLesson>> {
    admin.require(&state.policy, "course.manage")?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // SECURITY (XSS): sanitize lesson HTML at the write boundary before it
    // is ever read back by `{@html}` in the student view.
    if let Some(c) = req.content.as_deref() {
        req.content = Some(crate::common::html::sanitize_rich_text(c));
    }
    if let Some(d) = req.description.as_deref() {
        req.description = Some(crate::common::html::sanitize_plain_text(d));
    }

    // Verify module belongs to course
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM course_modules WHERE id = $1 AND course_id = $2")
        .bind(module_id)
        .bind(course_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Module not found".to_string()))?;

    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.title));
    let content = req.content.as_deref().unwrap_or("");
    let sort_order = req.sort_order.unwrap_or(0);
    let is_preview = req.is_preview.unwrap_or(false);

    let lesson = sqlx::query_as::<_, CourseLesson>(
        r#"
        INSERT INTO course_lessons
            (module_id, title, slug, description, content, content_json,
             video_url, video_duration_seconds, sort_order, is_preview)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, module_id, title, slug, description, content, content_json,
                  video_url, video_duration_seconds, sort_order, is_preview,
                  created_at, updated_at
        "#,
    )
    .bind(module_id)
    .bind(&req.title)
    .bind(&slug)
    .bind(&req.description)
    .bind(content)
    .bind(&req.content_json)
    .bind(&req.video_url)
    .bind(req.video_duration_seconds)
    .bind(sort_order)
    .bind(is_preview)
    .fetch_one(&state.db)
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.lesson.create",
        "course_lesson",
        lesson.id,
        serde_json::json!({
            "course_id": course_id,
            "module_id": module_id,
            "slug": lesson.slug,
        }),
    )
    .await;

    Ok(Json(lesson))
}

#[utoipa::path(
    put,
    path = "/api/admin/lessons/{lesson_id}",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("lesson_id" = Uuid, Path, description = "Lesson id")),
    request_body = UpdateLessonRequest,
    responses(
        (status = 200, description = "Lesson updated", body = CourseLesson),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Lesson not found")
    )
)]
pub(crate) async fn update_lesson(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(lesson_id): Path<Uuid>,
    Json(mut req): Json<UpdateLessonRequest>,
) -> AppResult<Json<CourseLesson>> {
    admin.require(&state.policy, "course.manage")?;
    // SECURITY (XSS): see `create_lesson` — sanitize at the write boundary.
    if let Some(c) = req.content.as_deref() {
        req.content = Some(crate::common::html::sanitize_rich_text(c));
    }
    if let Some(d) = req.description.as_deref() {
        req.description = Some(crate::common::html::sanitize_plain_text(d));
    }

    let slug = req.slug.as_deref().map(slugify);

    let lesson = sqlx::query_as::<_, CourseLesson>(
        r#"
        UPDATE course_lessons SET
            title = COALESCE($2, title),
            slug = COALESCE($3, slug),
            description = COALESCE($4, description),
            content = COALESCE($5, content),
            content_json = COALESCE($6, content_json),
            video_url = COALESCE($7, video_url),
            video_duration_seconds = COALESCE($8, video_duration_seconds),
            sort_order = COALESCE($9, sort_order),
            is_preview = COALESCE($10, is_preview),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, module_id, title, slug, description, content, content_json,
                  video_url, video_duration_seconds, sort_order, is_preview,
                  created_at, updated_at
        "#,
    )
    .bind(lesson_id)
    .bind(&req.title)
    .bind(&slug)
    .bind(&req.description)
    .bind(&req.content)
    .bind(&req.content_json)
    .bind(&req.video_url)
    .bind(req.video_duration_seconds)
    .bind(req.sort_order)
    .bind(req.is_preview)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Lesson not found".to_string()))?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.lesson.update",
        "course_lesson",
        lesson.id,
        serde_json::json!({
            "module_id": lesson.module_id,
            "slug": lesson.slug,
        }),
    )
    .await;

    Ok(Json(lesson))
}

#[utoipa::path(
    delete,
    path = "/api/admin/lessons/{lesson_id}",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("lesson_id" = Uuid, Path, description = "Lesson id")),
    responses(
        (status = 200, description = "Lesson deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Lesson not found")
    )
)]
pub(crate) async fn delete_lesson(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(lesson_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "course.manage")?;
    let rows = sqlx::query("DELETE FROM course_lessons WHERE id = $1")
        .bind(lesson_id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Lesson not found".to_string()));
    }

    audit_admin(
        &state.db,
        &admin,
        &client,
        "course.lesson.delete",
        "course_lesson",
        lesson_id,
        serde_json::json!({}),
    )
    .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Public Handlers ──────────────────────────────────────────────────────

async fn public_list_courses(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<CourseListItem>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM courses WHERE published = true")
        .fetch_one(&state.db)
        .await?;

    let courses = sqlx::query_as::<_, CourseListItem>(
        r#"
        SELECT c.id, c.title, c.slug, c.short_description, c.thumbnail_url,
               c.difficulty, u.name AS instructor_name, c.price_cents,
               c.is_free, c.is_included_in_subscription, c.published,
               c.estimated_duration_minutes,
               COUNT(cl.id)::bigint AS total_lessons,
               c.created_at
        FROM courses c
        JOIN users u ON u.id = c.instructor_id
        LEFT JOIN course_modules cm ON cm.course_id = c.id
        LEFT JOIN course_lessons cl ON cl.module_id = cm.id
        WHERE c.published = true
        GROUP BY c.id, u.name
        ORDER BY c.sort_order ASC, c.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total_pages = (total + per_page - 1) / per_page;

    Ok(Json(PaginatedResponse {
        data: courses,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_get_course(
    State(state): State<AppState>,
    MaybeAuthUser(auth): MaybeAuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<CourseWithModules>> {
    let course = sqlx::query_as::<_, Course>(
        r#"
        SELECT id, title, slug, description, short_description, thumbnail_url,
               trailer_video_url, difficulty, instructor_id, price_cents, currency,
               is_free, is_included_in_subscription, sort_order, published,
               published_at, estimated_duration_minutes, created_at, updated_at
        FROM courses
        WHERE slug = $1 AND published = true
        "#,
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".to_string()))?;

    // Decide whether the caller is allowed to see the FULL lesson body
    // (`content`, `content_json`, `video_url`). Free courses are open to
    // everyone; subscription-included courses require an active or trialing
    // subscription; admins always see everything for QA. Lessons flagged
    // `is_preview = TRUE` are public regardless — that's the marketing
    // teaser. Everything else returns with the body fields nulled out so
    // crawlers and unauthenticated visitors can browse the table of contents
    // without exfiltrating the paid content itself.
    let viewer_has_full_access = if course.is_free {
        true
    } else {
        match &auth {
            Some(au) if au.role == "admin" => true,
            Some(au) if course.is_included_in_subscription => {
                let sub = crate::db::find_subscription_by_user(&state.db, au.user_id).await?;
                matches!(
                    sub.as_ref().map(|s| s.status),
                    Some(crate::models::SubscriptionStatus::Active)
                        | Some(crate::models::SubscriptionStatus::Trialing)
                )
            }
            _ => false,
        }
    };

    let modules = sqlx::query_as::<_, CourseModule>(
        r#"
        SELECT id, course_id, title, description, sort_order, created_at, updated_at
        FROM course_modules
        WHERE course_id = $1
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(course.id)
    .fetch_all(&state.db)
    .await?;

    let lessons = sqlx::query_as::<_, CourseLesson>(
        r#"
        SELECT cl.id, cl.module_id, cl.title, cl.slug, cl.description,
               cl.content, cl.content_json, cl.video_url, cl.video_duration_seconds,
               cl.sort_order, cl.is_preview, cl.created_at, cl.updated_at
        FROM course_lessons cl
        JOIN course_modules cm ON cm.id = cl.module_id
        WHERE cm.course_id = $1
        ORDER BY cl.sort_order ASC, cl.created_at ASC
        "#,
    )
    .bind(course.id)
    .fetch_all(&state.db)
    .await?;

    // Total counts come from the unredacted set so the marketing card
    // surface ("12 lessons, 4h 20m") stays stable regardless of who is
    // looking at the course.
    let total_lessons = lessons.len() as i64;
    let total_duration_seconds: i64 = lessons
        .iter()
        .filter_map(|l| l.video_duration_seconds)
        .map(|d| d as i64)
        .sum();

    // Redact body fields on locked lessons. We keep `description`
    // (assumed to be marketing copy), `title`, `slug`, and metadata so the
    // SPA renders a "🔒 Subscribe to watch" card with everything it needs
    // except the paid payload itself.
    let lessons: Vec<CourseLesson> = lessons
        .into_iter()
        .map(|mut l| {
            if !viewer_has_full_access && !l.is_preview {
                l.content = String::new();
                l.content_json = None;
                l.video_url = None;
            }
            l
        })
        .collect();

    let mut modules_with_lessons: Vec<ModuleWithLessons> = modules
        .into_iter()
        .map(|m| ModuleWithLessons {
            module: m,
            lessons: Vec::new(),
        })
        .collect();

    for lesson in lessons {
        if let Some(mwl) = modules_with_lessons
            .iter_mut()
            .find(|mwl| mwl.module.id == lesson.module_id)
        {
            mwl.lessons.push(lesson);
        }
    }

    Ok(Json(CourseWithModules {
        course,
        modules: modules_with_lessons,
        total_lessons,
        total_duration_seconds,
    }))
}

// ── Member Handlers ──────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/member/courses/{course_id}/enroll",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("course_id" = Uuid, Path, description = "Course id")),
    responses(
        (status = 200, description = "Enrolled", body = CourseEnrollment),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Course not found")
    )
)]
pub(crate) async fn enroll_course(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(course_id): Path<Uuid>,
) -> AppResult<Json<CourseEnrollment>> {
    // Verify course exists, is published, and surface its access flags so we
    // can gate enrollment correctly. Three legitimate access modes today:
    //
    //   1. `is_free = TRUE`                      → anyone authenticated.
    //   2. `is_included_in_subscription = TRUE`  → requires active/trialing sub.
    //   3. `price_cents > 0` (à-la-carte)        → requires a prior purchase
    //      tracked in `course_purchases` (not implemented yet — explicit
    //      Forbidden until the purchase flow lands).
    //
    // Admins bypass the gate so they can preview courses without holding a
    // personal subscription.
    let row = sqlx::query_as::<_, (bool, bool, i64)>(
        "SELECT is_free, is_included_in_subscription, price_cents
           FROM courses WHERE id = $1 AND published = true",
    )
    .bind(course_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Course not found".to_string()))?;
    let (is_free, is_included_in_subscription, price_cents) = row;

    let is_admin = auth.role == "admin";

    if !is_free && !is_admin {
        if is_included_in_subscription {
            // Active or trialing sub is required.
            let sub = crate::db::find_subscription_by_user(&state.db, auth.user_id).await?;
            let allowed = matches!(
                sub.as_ref().map(|s| s.status),
                Some(crate::models::SubscriptionStatus::Active)
                    | Some(crate::models::SubscriptionStatus::Trialing)
            );
            if !allowed {
                return Err(AppError::Forbidden);
            }
        } else if price_cents > 0 {
            // Pay-per-course; no purchase ledger wired yet.
            return Err(AppError::Forbidden);
        }
    }

    let enrollment = sqlx::query_as::<_, CourseEnrollment>(
        r#"
        INSERT INTO course_enrollments (id, user_id, course_id, progress)
        VALUES ($1, $2, $3, 0)
        ON CONFLICT (user_id, course_id) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING id, user_id, course_id, progress, enrolled_at, completed_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.user_id)
    .bind(course_id.to_string())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(enrollment))
}

#[derive(serde::Serialize)]
struct CourseProgressResponse {
    enrollment: CourseEnrollment,
    lesson_progress: Vec<LessonProgress>,
}

async fn get_course_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(course_id): Path<Uuid>,
) -> AppResult<Json<CourseProgressResponse>> {
    let enrollment = sqlx::query_as::<_, CourseEnrollment>(
        r#"
        SELECT id, user_id, course_id, progress, enrolled_at, completed_at
        FROM course_enrollments
        WHERE user_id = $1 AND course_id = $2
        "#,
    )
    .bind(auth.user_id)
    .bind(course_id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Enrollment not found".to_string()))?;

    let lesson_progress = sqlx::query_as::<_, LessonProgress>(
        r#"
        SELECT lp.id, lp.user_id, lp.lesson_id, lp.completed, lp.progress_seconds,
               lp.completed_at, lp.last_accessed_at
        FROM lesson_progress lp
        JOIN course_lessons cl ON cl.id = lp.lesson_id
        JOIN course_modules cm ON cm.id = cl.module_id
        WHERE lp.user_id = $1 AND cm.course_id = $2
        "#,
    )
    .bind(auth.user_id)
    .bind(course_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(CourseProgressResponse {
        enrollment,
        lesson_progress,
    }))
}

#[utoipa::path(
    put,
    path = "/api/member/lessons/{lesson_id}/progress",
    tag = "courses",
    security(("bearer_auth" = [])),
    params(("lesson_id" = Uuid, Path, description = "Lesson id")),
    request_body = UpdateLessonProgressRequest,
    responses(
        (status = 200, description = "Progress updated", body = LessonProgress),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Lesson not found")
    )
)]
pub(crate) async fn update_lesson_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(lesson_id): Path<Uuid>,
    Json(req): Json<UpdateLessonProgressRequest>,
) -> AppResult<Json<LessonProgress>> {
    // Verify lesson exists
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM course_lessons WHERE id = $1")
        .bind(lesson_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Lesson not found".to_string()))?;

    let completed = req.completed.unwrap_or(false);
    let progress_seconds = req.progress_seconds.unwrap_or(0);
    let completed_at: Option<chrono::DateTime<Utc>> =
        if completed { Some(Utc::now()) } else { None };

    let progress = sqlx::query_as::<_, LessonProgress>(
        r#"
        INSERT INTO lesson_progress (user_id, lesson_id, completed, progress_seconds, completed_at, last_accessed_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (user_id, lesson_id) DO UPDATE SET
            completed = COALESCE($3, lesson_progress.completed),
            progress_seconds = COALESCE($4, lesson_progress.progress_seconds),
            completed_at = CASE
                WHEN $3 = true AND lesson_progress.completed_at IS NULL THEN NOW()
                ELSE lesson_progress.completed_at
            END,
            last_accessed_at = NOW()
        RETURNING id, user_id, lesson_id, completed, progress_seconds, completed_at, last_accessed_at
        "#,
    )
    .bind(auth.user_id)
    .bind(lesson_id)
    .bind(completed)
    .bind(progress_seconds)
    .bind(completed_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(progress))
}
