use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use bytes::Bytes;
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::{AdminUser, ClientInfo, PrivilegedUser},
    models::*,
    services::{
        audit::{audit_admin, audit_admin_priv},
        MediaBackend, R2Storage,
    },
    AppState,
};

/// Resolve which permission key to enforce for a post-mutation handler
/// when an actor is operating on `existing.author_id`.
///
/// The FDN-07 matrix splits blog mutators into `_own` and `_any` variants
/// (021_rbac.sql:62-72). `author` carries the `_own` set; `admin` carries
/// both. This helper picks the correct key based on whether the actor
/// authored the post in question, then delegates to `policy.require()`
/// for the actual gate.
///
/// Forensic Wave-2 PR-7 (C-7): until this helper landed, every blog
/// mutator hard-coded `_any`, which made the `_own` permissions dead
/// code and meant authors got 403 on every mutation except create.
fn require_blog_post_action(
    policy: &crate::authz::PolicyHandle,
    actor: &PrivilegedUser,
    existing: &BlogPost,
    base_action: &str,
) -> AppResult<()> {
    let key = if existing.author_id == actor.user_id {
        format!("{base_action}_own")
    } else {
        format!("{base_action}_any")
    };
    if policy.has(actor.role, &key) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

// ── Admin Blog Router ──────────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        // Posts
        .route("/posts", get(admin_list_posts))
        .route("/posts", post(admin_create_post))
        .route("/posts/{id}", get(admin_get_post))
        .route("/posts/{id}", put(admin_update_post))
        .route("/posts/{id}", delete(admin_delete_post))
        .route("/posts/{id}/restore", post(admin_restore_post_from_trash))
        .route("/posts/{id}/status", put(admin_update_post_status))
        .route("/posts/{id}/autosave", post(admin_autosave_post))
        .route("/posts/{id}/revisions", get(admin_list_revisions))
        .route(
            "/posts/{id}/revisions/{rev_id}/restore",
            post(admin_restore_revision),
        )
        .route("/posts/{id}/meta", get(admin_list_post_meta))
        .route("/posts/{id}/meta", post(admin_upsert_post_meta))
        .route("/posts/{id}/meta/{key}", delete(admin_delete_post_meta))
        // Categories
        .route("/categories", get(admin_list_categories))
        .route("/categories", post(admin_create_category))
        .route("/categories/{id}", put(admin_update_category))
        .route("/categories/{id}", delete(admin_delete_category))
        // Tags
        .route("/tags", get(admin_list_tags))
        .route("/tags", post(admin_create_tag))
        .route("/tags/{id}", delete(admin_delete_tag))
        // Media
        .route("/media", get(admin_list_media))
        .route("/media/upload", post(admin_upload_media))
        .route("/media/{id}", put(admin_update_media))
        .route("/media/{id}", delete(admin_delete_media))
}

// ── Public Blog Router ─────────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/posts", get(public_list_posts))
        .route("/posts/{slug}", get(public_get_post))
        .route("/posts/{slug}/unlock", post(public_unlock_post))
        .route("/categories", get(public_list_categories))
        .route("/tags", get(public_list_tags))
        .route("/posts/category/{slug}", get(public_posts_by_category))
        .route("/posts/tag/{slug}", get(public_posts_by_tag))
        .route("/slugs", get(public_all_slugs))
}

// ── Helper: Build full post response ───────────────────────────────────

async fn build_post_response(pool: &sqlx::PgPool, post: BlogPost) -> AppResult<BlogPostResponse> {
    let author = db::find_user_by_id(pool, post.author_id)
        .await?
        .ok_or(AppError::NotFound("Author not found".to_string()))?;
    let categories = db::get_categories_for_post(pool, post.id).await?;
    let tags = db::get_tags_for_post(pool, post.id).await?;
    let meta = db::list_post_meta(pool, post.id).await?;
    let featured_image_url = if let Some(img_id) = post.featured_image_id {
        db::get_media(pool, img_id).await?.map(|m| m.url)
    } else {
        None
    };

    Ok(BlogPostResponse {
        id: post.id,
        author_id: post.author_id,
        author_name: author.name.clone(),
        author_avatar: author.avatar_url.clone(),
        author_position: author.position.clone(),
        author_bio: author.bio.clone(),
        author_website: author.website_url.clone(),
        author_twitter: author.twitter_url.clone(),
        author_linkedin: author.linkedin_url.clone(),
        author_youtube: author.youtube_url.clone(),
        title: post.title,
        slug: post.slug,
        content: post.content,
        content_json: post.content_json,
        excerpt: post.excerpt,
        featured_image_url,
        status: post.status,
        pre_trash_status: post.pre_trash_status,
        trashed_at: post.trashed_at,
        visibility: post.visibility.clone(),
        is_password_protected: post.password_hash.is_some(),
        format: post.format.clone(),
        is_sticky: post.is_sticky,
        allow_comments: post.allow_comments,
        meta_title: post.meta_title,
        meta_description: post.meta_description,
        canonical_url: post.canonical_url,
        og_image_url: post.og_image_url,
        reading_time_minutes: post.reading_time_minutes,
        word_count: post.word_count,
        categories,
        tags,
        meta,
        scheduled_at: post.scheduled_at,
        published_at: post.published_at,
        created_at: post.created_at,
        updated_at: post.updated_at,
    })
}

async fn build_post_list_item(pool: &sqlx::PgPool, post: BlogPost) -> AppResult<BlogPostListItem> {
    let author = db::find_user_by_id(pool, post.author_id).await?;
    let author_name = author
        .map(|a| a.name)
        .unwrap_or_else(|| "Unknown".to_string());
    let categories = db::get_categories_for_post(pool, post.id).await?;
    let tags = db::get_tags_for_post(pool, post.id).await?;
    let featured_image_url = if let Some(img_id) = post.featured_image_id {
        db::get_media(pool, img_id).await?.map(|m| m.url)
    } else {
        None
    };

    Ok(BlogPostListItem {
        id: post.id,
        author_id: post.author_id,
        author_name,
        title: post.title,
        slug: post.slug,
        excerpt: post.excerpt,
        featured_image_url,
        status: post.status,
        format: post.format,
        is_sticky: post.is_sticky,
        reading_time_minutes: post.reading_time_minutes,
        word_count: post.word_count,
        published_at: post.published_at,
        created_at: post.created_at,
        updated_at: post.updated_at,
        categories,
        tags,
    })
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN POST HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_posts(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PostListParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let (posts, total) = db::list_blog_posts_admin(
        &state.db,
        offset,
        per_page,
        params.status.as_ref(),
        params.author_id,
        params.search.as_deref(),
    )
    .await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

fn hash_post_password(plain: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {e}")))
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/posts",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    request_body = CreatePostRequest,
    responses(
        (status = 200, description = "Post created", body = BlogPostResponse),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn admin_create_post(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(mut req): Json<CreatePostRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    admin.require(&state.policy, "blog.post.create")?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // SECURITY (XSS): sanitize author-supplied HTML at the write boundary so
    // every downstream render (server SSR, `{@html}` in Svelte, RSS/Atom,
    // email digests) sees a safe, ammonia-cleaned string. Defence-in-depth:
    // the frontend still applies DOMPurify before `{@html}`.
    if let Some(c) = req.content.as_deref() {
        req.content = Some(crate::common::html::sanitize_rich_text(c));
    }
    if let Some(e) = req.excerpt.as_deref() {
        req.excerpt = Some(crate::common::html::sanitize_plain_text(e));
    }

    let password_hash = if req.visibility.as_deref() == Some("password") {
        if let Some(ref pw) = req.post_password {
            if !pw.is_empty() {
                Some(hash_post_password(pw)?)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let effective_author = req.author_id.unwrap_or(admin.user_id);
    let post =
        db::create_blog_post(&state.db, effective_author, &req, password_hash.as_deref()).await?;

    // Set categories/tags if provided
    if let Some(ref cat_ids) = req.category_ids {
        db::set_post_categories(&state.db, post.id, cat_ids).await?;
    }
    if let Some(ref tag_ids) = req.tag_ids {
        db::set_post_tags(&state.db, post.id, tag_ids).await?;
    }

    let response = build_post_response(&state.db, post).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.post.create",
        "blog_post",
        response.id,
        serde_json::json!({
            "slug": response.slug,
            "status": response.status,
        }),
    )
    .await;

    Ok(Json(response))
}

async fn admin_get_post(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/api/admin/blog/posts/{id}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Post id")),
    request_body = UpdatePostRequest,
    responses(
        (status = 200, description = "Post updated", body = BlogPostResponse),
        (status = 400, description = "Post in trash"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Post not found")
    )
)]
pub(crate) async fn admin_update_post(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(mut req): Json<UpdatePostRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    // Ownership-aware RBAC: authors satisfy `blog.post.update_own` on their
    // own posts; admins (who hold both keys) reach any post under
    // `blog.post.update_any`. Resolved against the freshly-read `existing`
    // row so a stale ownership claim cannot privilege escalate.
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    require_blog_post_action(&state.policy, &admin, &existing, "blog.post.update")?;

    // SECURITY (XSS): sanitize author-supplied HTML at the write boundary —
    // see `admin_create_post` for the full rationale.
    if let Some(c) = req.content.as_deref() {
        req.content = Some(crate::common::html::sanitize_rich_text(c));
    }
    if let Some(e) = req.excerpt.as_deref() {
        req.excerpt = Some(crate::common::html::sanitize_plain_text(e));
    }

    if existing.status == PostStatus::Trash {
        if let Some(ref s) = req.status {
            if s != &PostStatus::Trash {
                return Err(AppError::BadRequest(
                    "This post is in the trash. Restore it first (POST .../restore), or keep status as trash."
                        .to_string(),
                ));
            }
        }
    }

    // Create revision before updating
    db::create_blog_revision(
        &state.db,
        id,
        admin.user_id,
        &existing.title,
        &existing.content,
        existing.content_json.as_ref(),
    )
    .await?;

    let password_hash_update: Option<Option<String>> = if let Some(vis) = req.visibility.as_deref()
    {
        if vis == "password" {
            if let Some(ref pw) = req.post_password {
                if !pw.is_empty() {
                    Some(Some(hash_post_password(pw)?))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            Some(None)
        }
    } else if let Some(ref pw) = req.post_password {
        if !pw.is_empty() {
            Some(Some(hash_post_password(pw)?))
        } else {
            None
        }
    } else {
        None
    };

    let post =
        db::update_blog_post(&state.db, id, &req, password_hash_update, req.author_id).await?;

    // Update categories/tags if provided
    if let Some(ref cat_ids) = req.category_ids {
        db::set_post_categories(&state.db, post.id, cat_ids).await?;
    }
    if let Some(ref tag_ids) = req.tag_ids {
        db::set_post_tags(&state.db, post.id, tag_ids).await?;
    }

    let response = build_post_response(&state.db, post).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "blog.post.update",
        "blog_post",
        response.id,
        serde_json::json!({
            "slug": response.slug,
            "status": response.status,
            "owned_by_actor": existing.author_id == admin.user_id,
        }),
    )
    .await;

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/admin/blog/posts/{id}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Post id")),
    responses(
        (status = 200, description = "Post permanently deleted"),
        (status = 400, description = "Post not in trash"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Post not found")
    )
)]
pub(crate) async fn admin_delete_post(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    require_blog_post_action(&state.policy, &admin, &existing, "blog.post.delete")?;

    if existing.status != PostStatus::Trash {
        return Err(AppError::BadRequest(
            "Only posts in the trash can be permanently deleted. Move the post to trash first."
                .to_string(),
        ));
    }
    db::delete_blog_post(&state.db, id).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "blog.post.delete",
        "blog_post",
        id,
        serde_json::json!({
            "slug": existing.slug,
            "title": existing.title,
            "owned_by_actor": existing.author_id == admin.user_id,
        }),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "message": "Post permanently deleted" }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/posts/{id}/restore",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Post id")),
    responses(
        (status = 200, description = "Post restored from trash", body = BlogPostResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_restore_post_from_trash(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    require_blog_post_action(&state.policy, &admin, &existing, "blog.post.update")?;

    let post = db::restore_post_from_trash(&state.db, id).await?;
    let response = build_post_response(&state.db, post).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "blog.post.restore",
        "blog_post",
        response.id,
        serde_json::json!({
            "slug": response.slug,
            "status": response.status,
            "owned_by_actor": existing.author_id == admin.user_id,
        }),
    )
    .await;

    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/api/admin/blog/posts/{id}/status",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Post id")),
    request_body = UpdatePostStatusRequest,
    responses(
        (status = 200, description = "Post status updated", body = BlogPostResponse),
        (status = 400, description = "Post in trash"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Post not found")
    )
)]
pub(crate) async fn admin_update_post_status(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePostStatusRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    // The `publish` key has no `_own/_any` split (021_rbac.sql:69), so we
    // pair it with an explicit ownership check: the actor must hold
    // `blog.post.publish` AND either own the post or hold `update_any`.
    // This lets author publish their own posts while preventing them
    // from publishing another author's draft.
    if !state.policy.has(admin.role, "blog.post.publish") {
        return Err(AppError::Forbidden);
    }
    if existing.author_id != admin.user_id && !state.policy.has(admin.role, "blog.post.update_any")
    {
        return Err(AppError::Forbidden);
    }

    let post = if req.status == PostStatus::Trash {
        db::move_post_to_trash(&state.db, id).await?
    } else if existing.status == PostStatus::Trash {
        return Err(AppError::BadRequest(
            "This post is in the trash. Use POST /api/admin/blog/posts/{id}/restore to restore it."
                .to_string(),
        ));
    } else {
        db::update_post_status(&state.db, id, &req.status).await?
    };
    let response = build_post_response(&state.db, post).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "blog.post.status.update",
        "blog_post",
        response.id,
        serde_json::json!({
            "slug": response.slug,
            "from": existing.status,
            "to": req.status,
            "owned_by_actor": existing.author_id == admin.user_id,
        }),
    )
    .await;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/posts/{id}/autosave",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Post id")),
    request_body = AutosaveRequest,
    responses(
        (status = 200, description = "Autosave recorded"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_autosave_post(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AutosaveRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    require_blog_post_action(&state.policy, &admin, &existing, "blog.post.update")?;

    db::autosave_blog_post(&state.db, id, &req).await?;
    Ok(Json(serde_json::json!({ "message": "Autosaved" })))
}

async fn admin_list_revisions(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<RevisionResponse>>> {
    let revisions = db::list_blog_revisions(&state.db, id).await?;
    let mut items = Vec::with_capacity(revisions.len());
    for rev in revisions {
        let author = db::find_user_by_id(&state.db, rev.author_id).await?;
        let author_name = author
            .map(|a| a.name)
            .unwrap_or_else(|| "Unknown".to_string());
        items.push(RevisionResponse {
            id: rev.id,
            post_id: rev.post_id,
            author_id: rev.author_id,
            author_name,
            title: rev.title,
            revision_number: rev.revision_number,
            created_at: rev.created_at,
        });
    }
    Ok(Json(items))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RevisionRestorePath {
    id: Uuid,
    rev_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/posts/{id}/revisions/{rev_id}/restore",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Post id"),
        ("rev_id" = Uuid, Path, description = "Revision id"),
    ),
    responses(
        (status = 200, description = "Post restored from revision", body = BlogPostResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Post or revision not found")
    )
)]
pub(crate) async fn admin_restore_revision(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(path): Path<RevisionRestorePath>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, path.id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    require_blog_post_action(&state.policy, &admin, &existing, "blog.post.update")?;

    let revision = db::get_blog_revision(&state.db, path.rev_id)
        .await?
        .ok_or(AppError::NotFound("Revision not found".to_string()))?;

    // Create revision of current state before restoring
    db::create_blog_revision(
        &state.db,
        path.id,
        admin.user_id,
        &existing.title,
        &existing.content,
        existing.content_json.as_ref(),
    )
    .await?;

    // Restore from revision
    let req = UpdatePostRequest {
        title: Some(revision.title),
        content: Some(revision.content),
        content_json: revision.content_json,
        slug: None,
        excerpt: None,
        featured_image_id: None,
        status: None,
        visibility: None,
        post_password: None,
        is_sticky: None,
        allow_comments: None,
        meta_title: None,
        meta_description: None,
        canonical_url: None,
        og_image_url: None,
        category_ids: None,
        tag_ids: None,
        scheduled_at: None,
        author_id: None,
        format: None,
    };

    let post = db::update_blog_post(&state.db, path.id, &req, None, None).await?;
    let response = build_post_response(&state.db, post).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "blog.post.revision.restore",
        "blog_post",
        response.id,
        serde_json::json!({
            "slug": response.slug,
            "revision_id": path.rev_id,
            "revision_number": revision.revision_number,
            "owned_by_actor": existing.author_id == admin.user_id,
        }),
    )
    .await;

    Ok(Json(response))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN CATEGORY HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_categories(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<BlogCategory>>> {
    let cats = db::list_blog_categories(&state.db).await?;
    Ok(Json(cats))
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/categories",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    request_body = CreateCategoryRequest,
    responses(
        (status = 200, description = "Category created", body = BlogCategory),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn admin_create_category(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<CreateCategoryRequest>,
) -> AppResult<Json<BlogCategory>> {
    admin.require(&state.policy, "blog.category.manage")?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let cat = db::create_blog_category(&state.db, &req).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.category.create",
        "blog_category",
        cat.id,
        serde_json::json!({ "slug": cat.slug, "name": cat.name }),
    )
    .await;

    Ok(Json(cat))
}

#[utoipa::path(
    put,
    path = "/api/admin/blog/categories/{id}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Category id")),
    request_body = UpdateCategoryRequest,
    responses(
        (status = 200, description = "Category updated", body = BlogCategory),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_update_category(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategoryRequest>,
) -> AppResult<Json<BlogCategory>> {
    admin.require(&state.policy, "blog.category.manage")?;
    let cat = db::update_blog_category(&state.db, id, &req).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.category.update",
        "blog_category",
        cat.id,
        serde_json::json!({ "slug": cat.slug, "name": cat.name }),
    )
    .await;

    Ok(Json(cat))
}

#[utoipa::path(
    delete,
    path = "/api/admin/blog/categories/{id}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Category id")),
    responses(
        (status = 200, description = "Category deleted"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_delete_category(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "blog.category.manage")?;
    db::delete_blog_category(&state.db, id).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.category.delete",
        "blog_category",
        id,
        serde_json::json!({}),
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Category deleted" })))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN TAG HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_tags(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<BlogTag>>> {
    let tags = db::list_blog_tags(&state.db).await?;
    Ok(Json(tags))
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/tags",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    request_body = CreateTagRequest,
    responses(
        (status = 200, description = "Tag created", body = BlogTag),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn admin_create_tag(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<CreateTagRequest>,
) -> AppResult<Json<BlogTag>> {
    admin.require(&state.policy, "blog.category.manage")?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let tag = db::create_blog_tag(&state.db, &req).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.tag.create",
        "blog_tag",
        tag.id,
        serde_json::json!({ "slug": tag.slug, "name": tag.name }),
    )
    .await;

    Ok(Json(tag))
}

#[utoipa::path(
    delete,
    path = "/api/admin/blog/tags/{id}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tag id")),
    responses(
        (status = 200, description = "Tag deleted"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_delete_tag(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "blog.category.manage")?;
    db::delete_blog_tag(&state.db, id).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.tag.delete",
        "blog_tag",
        id,
        serde_json::json!({}),
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Tag deleted" })))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN MEDIA HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_media(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Media>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (items, total) = db::list_media(&state.db, offset, per_page).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/media/upload",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Media uploaded", body = Media),
        (status = 400, description = "Invalid file or multipart error"),
        (status = 403, description = "Forbidden"),
        (status = 413, description = "Upload exceeds size cap")
    )
)]
pub(crate) async fn admin_upload_media(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    mut multipart: Multipart,
) -> AppResult<Json<Media>> {
    admin.require(&state.policy, "blog.media.upload")?;
    let api_url = &state.config.api_url;

    let mut file_data: Option<Vec<u8>> = None;
    let mut original_filename = String::new();
    let mut content_type = String::new();
    let mut title: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            original_filename = field.file_name().unwrap_or("unknown").to_string();
            content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read file: {}", e)))?;
            file_data = Some(data.to_vec());
        } else if name == "title" {
            let text = field
                .text()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read title: {}", e)))?;
            if !text.trim().is_empty() {
                title = Some(text.trim().to_string());
            }
        }
    }

    let data = file_data.ok_or(AppError::BadRequest("No file provided".to_string()))?;

    // Enforce an explicit cap at the handler boundary. Matches Phase 3 §12 file-upload hardening.
    const MAX_MEDIA_UPLOAD_BYTES: usize = 10 * 1024 * 1024;
    if data.len() > MAX_MEDIA_UPLOAD_BYTES {
        return Err(AppError::PayloadTooLarge(format!(
            "Upload exceeds the {} MB limit",
            MAX_MEDIA_UPLOAD_BYTES / (1024 * 1024)
        )));
    }

    // Validate MIME type
    let allowed = [
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "image/avif",
        "image/svg+xml",
        "application/pdf",
    ];
    if !allowed.contains(&content_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "File type '{}' not allowed",
            content_type
        )));
    }

    let file_size = data.len() as i64;

    let (storage_path, url, stored_filename) = match &state.media_backend {
        MediaBackend::R2(r2) => {
            let key = R2Storage::generate_key(&original_filename);
            let url = r2.upload(&key, Bytes::from(data), &content_type).await?;
            let stored_filename = key.rsplit('/').next().unwrap_or(key.as_str()).to_string();
            (key, url, stored_filename)
        }
        MediaBackend::Local { upload_dir } => {
            tokio::fs::create_dir_all(upload_dir)
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to create upload dir: {}", e)))?;

            let safe_name = sanitize_filename::sanitize(&original_filename);
            let ext = std::path::Path::new(&safe_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin");
            let unique_name = format!("{}.{}", Uuid::new_v4(), ext);
            let storage_path = format!("{}/{}", upload_dir, unique_name);
            let url = format!("{}/uploads/{}", api_url, unique_name);

            tokio::fs::write(&storage_path, &data)
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to write file: {}", e)))?;

            (storage_path, url, unique_name)
        }
    };

    // Get image dimensions if applicable
    let (width, height) = if content_type.starts_with("image/") && content_type != "image/svg+xml" {
        // Simple dimension detection: read first bytes
        // For now, store None; could add `image` crate later
        (None, None)
    } else {
        (None, None)
    };

    let media = db::create_media(
        &state.db,
        admin.user_id,
        &stored_filename,
        &original_filename,
        title.as_deref(),
        &content_type,
        file_size,
        width,
        height,
        &storage_path,
        &url,
    )
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.media.create",
        "media",
        media.id,
        serde_json::json!({
            "filename": media.original_filename,
            "mime_type": media.mime_type,
            "size_bytes": media.file_size,
        }),
    )
    .await;

    Ok(Json(media))
}

#[utoipa::path(
    put,
    path = "/api/admin/blog/media/{id}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Media id")),
    request_body = UpdateMediaRequest,
    responses(
        (status = 200, description = "Media metadata updated", body = Media),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_update_media(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMediaRequest>,
) -> AppResult<Json<Media>> {
    admin.require(&state.policy, "blog.media.upload")?;
    let media = db::update_media(&state.db, id, &req).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.media.update",
        "media",
        media.id,
        serde_json::json!({
            "filename": media.original_filename,
        }),
    )
    .await;

    Ok(Json(media))
}

#[utoipa::path(
    delete,
    path = "/api/admin/blog/media/{id}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Media id")),
    responses(
        (status = 200, description = "Media deleted"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_delete_media(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "blog.media.delete_any")?;
    let media = db::delete_media(&state.db, id).await?;

    if let Some(m) = &media {
        match &state.media_backend {
            MediaBackend::R2(r2) if m.storage_path.starts_with("media/") => {
                if let Err(e) = r2.delete_object(&m.storage_path).await {
                    tracing::warn!(
                        key = %m.storage_path,
                        error = %e,
                        "Failed to delete object from R2 (metadata already removed)"
                    );
                }
            }
            _ => {
                let _ = tokio::fs::remove_file(&m.storage_path).await;
            }
        }
    }

    audit_admin(
        &state.db,
        &admin,
        &client,
        "blog.media.delete",
        "media",
        id,
        serde_json::json!({
            "filename": media.as_ref().map(|m| m.original_filename.clone()),
        }),
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Media deleted" })))
}

// ══════════════════════════════════════════════════════════════════════
// PUBLIC BLOG HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn public_list_posts(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page();
    let page = params.page.unwrap_or(1).max(1);
    let offset = params.offset();

    let (posts, total) = db::list_published_posts(&state.db, offset, per_page).await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_get_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::get_blog_post_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let mut response = build_post_response(&state.db, post).await?;
    if response.is_password_protected {
        response.content = String::new();
        response.content_json = None;
    }
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/blog/posts/{slug}/unlock",
    tag = "blog",
    params(("slug" = String, Path, description = "Post slug")),
    request_body = VerifyPostPasswordRequest,
    responses(
        (status = 200, description = "Password accepted; full post returned", body = BlogPostResponse),
        (status = 400, description = "Post is not password protected"),
        (status = 401, description = "Invalid password"),
        (status = 404, description = "Post not found")
    )
)]
pub(crate) async fn public_unlock_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<VerifyPostPasswordRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::get_blog_post_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let hash_str = post
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("Post is not password protected".to_string()))?;

    let parsed = PasswordHash::new(hash_str)
        .map_err(|_| AppError::BadRequest("Invalid password hash".to_string()))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .map_err(|_| AppError::Unauthorized)?;

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn public_list_categories(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<BlogCategory>>> {
    let cats = db::list_blog_categories(&state.db).await?;
    Ok(Json(cats))
}

async fn public_list_tags(State(state): State<AppState>) -> AppResult<Json<Vec<BlogTag>>> {
    let tags = db::list_blog_tags(&state.db).await?;
    Ok(Json(tags))
}

async fn public_posts_by_category(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page();
    let page = params.page.unwrap_or(1).max(1);
    let offset = params.offset();

    let (posts, total) =
        db::list_published_posts_by_category(&state.db, &slug, offset, per_page).await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_posts_by_tag(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page();
    let page = params.page.unwrap_or(1).max(1);
    let offset = params.offset();

    let (posts, total) =
        db::list_published_posts_by_tag(&state.db, &slug, offset, per_page).await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_all_slugs(State(state): State<AppState>) -> AppResult<Json<Vec<String>>> {
    let slugs = db::list_all_published_slugs(&state.db).await?;
    Ok(Json(slugs))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN POST META HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_post_meta(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<PostMeta>>> {
    let items = db::list_post_meta(&state.db, id).await?;
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api/admin/blog/posts/{id}/meta",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Post id")),
    request_body = UpsertPostMetaRequest,
    responses(
        (status = 200, description = "Meta upserted", body = PostMeta),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_upsert_post_meta(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpsertPostMetaRequest>,
) -> AppResult<Json<PostMeta>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    require_blog_post_action(&state.policy, &admin, &existing, "blog.post.update")?;

    let item = db::upsert_post_meta(&state.db, id, &req.meta_key, &req.meta_value).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "blog.post_meta.upsert",
        "blog_post_meta",
        item.id,
        serde_json::json!({
            "post_id": id,
            "meta_key": req.meta_key,
            "owned_by_actor": existing.author_id == admin.user_id,
        }),
    )
    .await;

    Ok(Json(item))
}

#[utoipa::path(
    delete,
    path = "/api/admin/blog/posts/{id}/meta/{key}",
    tag = "admin-blog",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Post id"),
        ("key" = String, Path, description = "Meta key to delete"),
    ),
    responses(
        (status = 204, description = "Meta deleted"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_delete_post_meta(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path((id, key)): Path<(Uuid, String)>,
) -> AppResult<axum::http::StatusCode> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    require_blog_post_action(&state.policy, &admin, &existing, "blog.post.update")?;

    db::delete_post_meta(&state.db, id, &key).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "blog.post_meta.delete",
        "blog_post_meta",
        format!("{}:{}", id, key),
        serde_json::json!({
            "post_id": id,
            "meta_key": key,
            "owned_by_actor": existing.author_id == admin.user_id,
        }),
    )
    .await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
