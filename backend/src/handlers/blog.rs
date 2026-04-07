use axum::{
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AdminUser,
    models::*,
    AppState,
};

// ── Admin Blog Router ──────────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        // Posts
        .route("/posts", get(admin_list_posts))
        .route("/posts", post(admin_create_post))
        .route("/posts/{id}", get(admin_get_post))
        .route("/posts/{id}", put(admin_update_post))
        .route("/posts/{id}", delete(admin_delete_post))
        .route("/posts/{id}/status", put(admin_update_post_status))
        .route("/posts/{id}/autosave", post(admin_autosave_post))
        .route("/posts/{id}/revisions", get(admin_list_revisions))
        .route("/posts/{id}/revisions/{rev_id}/restore", post(admin_restore_revision))
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
        .route("/categories", get(public_list_categories))
        .route("/tags", get(public_list_tags))
        .route("/posts/category/{slug}", get(public_posts_by_category))
        .route("/posts/tag/{slug}", get(public_posts_by_tag))
        .route("/slugs", get(public_all_slugs))
}

// ── Helper: Build full post response ───────────────────────────────────

async fn build_post_response(
    pool: &sqlx::PgPool,
    post: BlogPost,
) -> AppResult<BlogPostResponse> {
    let author = db::find_user_by_id(pool, post.author_id)
        .await?
        .ok_or(AppError::NotFound("Author not found".to_string()))?;
    let categories = db::get_categories_for_post(pool, post.id).await?;
    let tags = db::get_tags_for_post(pool, post.id).await?;
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
        visibility: post.visibility,
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
        scheduled_at: post.scheduled_at,
        published_at: post.published_at,
        created_at: post.created_at,
        updated_at: post.updated_at,
    })
}

async fn build_post_list_item(
    pool: &sqlx::PgPool,
    post: BlogPost,
) -> AppResult<BlogPostListItem> {
    let author = db::find_user_by_id(pool, post.author_id).await?;
    let author_name = author.map(|a| a.name).unwrap_or_else(|| "Unknown".to_string());
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
    let per_page = params.per_page.unwrap_or(20).min(100).max(1);
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

async fn admin_create_post(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreatePostRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let post = db::create_blog_post(&state.db, admin.user_id, &req).await?;

    // Set categories/tags if provided
    if let Some(ref cat_ids) = req.category_ids {
        db::set_post_categories(&state.db, post.id, cat_ids).await?;
    }
    if let Some(ref tag_ids) = req.tag_ids {
        db::set_post_tags(&state.db, post.id, tag_ids).await?;
    }

    let response = build_post_response(&state.db, post).await?;
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

async fn admin_update_post(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePostRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

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

    let post = db::update_blog_post(&state.db, id, &req).await?;

    // Update categories/tags if provided
    if let Some(ref cat_ids) = req.category_ids {
        db::set_post_categories(&state.db, post.id, cat_ids).await?;
    }
    if let Some(ref tag_ids) = req.tag_ids {
        db::set_post_tags(&state.db, post.id, tag_ids).await?;
    }

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn admin_delete_post(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_blog_post(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Post deleted" })))
}

async fn admin_update_post_status(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePostStatusRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::update_post_status(&state.db, id, &req.status).await?;
    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn admin_autosave_post(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AutosaveRequest>,
) -> AppResult<Json<serde_json::Value>> {
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
        let author_name = author.map(|a| a.name).unwrap_or_else(|| "Unknown".to_string());
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

#[derive(serde::Deserialize)]
struct RevisionRestorePath {
    id: Uuid,
    rev_id: Uuid,
}

async fn admin_restore_revision(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(path): Path<RevisionRestorePath>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, path.id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

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
        is_sticky: None,
        allow_comments: None,
        meta_title: None,
        meta_description: None,
        canonical_url: None,
        og_image_url: None,
        category_ids: None,
        tag_ids: None,
        scheduled_at: None,
    };

    let post = db::update_blog_post(&state.db, path.id, &req).await?;
    let response = build_post_response(&state.db, post).await?;
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

async fn admin_create_category(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateCategoryRequest>,
) -> AppResult<Json<BlogCategory>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let cat = db::create_blog_category(&state.db, &req).await?;
    Ok(Json(cat))
}

async fn admin_update_category(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategoryRequest>,
) -> AppResult<Json<BlogCategory>> {
    let cat = db::update_blog_category(&state.db, id, &req).await?;
    Ok(Json(cat))
}

async fn admin_delete_category(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_blog_category(&state.db, id).await?;
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

async fn admin_create_tag(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateTagRequest>,
) -> AppResult<Json<BlogTag>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let tag = db::create_blog_tag(&state.db, &req).await?;
    Ok(Json(tag))
}

async fn admin_delete_tag(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_blog_tag(&state.db, id).await?;
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

async fn admin_upload_media(
    State(state): State<AppState>,
    admin: AdminUser,
    mut multipart: Multipart,
) -> AppResult<Json<Media>> {
    let upload_dir = &state.config.upload_dir;
    let api_url = &state.config.api_url;

    // Ensure upload dir exists
    tokio::fs::create_dir_all(upload_dir)
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to create upload dir: {}", e)))?;

    let mut file_data: Option<Vec<u8>> = None;
    let mut original_filename = String::new();
    let mut content_type = String::new();
    let mut title: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Multipart error: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            original_filename = field
                .file_name()
                .unwrap_or("unknown")
                .to_string();
            content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let data = field.bytes().await.map_err(|e| {
                AppError::BadRequest(format!("Failed to read file: {}", e))
            })?;
            file_data = Some(data.to_vec());
        } else if name == "title" {
            let text = field.text().await.map_err(|e| {
                AppError::BadRequest(format!("Failed to read title: {}", e))
            })?;
            if !text.trim().is_empty() {
                title = Some(text.trim().to_string());
            }
        }
    }

    let data = file_data.ok_or(AppError::BadRequest("No file provided".to_string()))?;

    // Validate MIME type
    let allowed = [
        "image/jpeg", "image/png", "image/gif", "image/webp", "image/avif", "image/svg+xml",
        "application/pdf",
    ];
    if !allowed.contains(&content_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "File type '{}' not allowed",
            content_type
        )));
    }

    let file_size = data.len() as i64;
    let safe_name = sanitize_filename::sanitize(&original_filename);
    let ext = std::path::Path::new(&safe_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    let unique_name = format!("{}.{}", Uuid::new_v4(), ext);
    let storage_path = format!("{}/{}", upload_dir, unique_name);
    let url = format!("{}/uploads/{}", api_url, unique_name);

    // Write file
    tokio::fs::write(&storage_path, &data)
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to write file: {}", e)))?;

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
        &unique_name,
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

    Ok(Json(media))
}

async fn admin_update_media(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMediaRequest>,
) -> AppResult<Json<Media>> {
    let media = db::update_media(&state.db, id, &req).await?;
    Ok(Json(media))
}

async fn admin_delete_media(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let media = db::delete_media(&state.db, id).await?;

    // Delete file from disk
    if let Some(m) = media {
        let _ = tokio::fs::remove_file(&m.storage_path).await;
    }

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

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn public_list_categories(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<BlogCategory>>> {
    let cats = db::list_blog_categories(&state.db).await?;
    Ok(Json(cats))
}

async fn public_list_tags(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<BlogTag>>> {
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

async fn public_all_slugs(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<String>>> {
    let slugs = db::list_all_published_slugs(&state.db).await?;
    Ok(Json(slugs))
}
