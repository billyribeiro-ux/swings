import type { CreatePostPayload, PostStatus, UpdatePostPayload } from '$lib/api/types';

export interface PostEditorPayloadInput {
	title: string;
	slug: string;
	content: string;
	contentJson: Record<string, unknown> | null;
	excerpt: string;
	featuredImageId?: string | undefined;
	status: PostStatus;
	visibility: string;
	isSticky: boolean;
	allowComments: boolean;
	metaTitle: string;
	metaDescription: string;
	canonicalUrl: string;
	ogImageUrl: string;
	selectedCategoryIds: string[];
	selectedTagIds: string[];
	scheduledAt: string;
	postPassword: string;
	authorId: string;
	postFormat: string;
}

export function slugifyPostTitle(input: string): string {
	return input
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '');
}

export function buildPostPayload(
	input: PostEditorPayloadInput
): CreatePostPayload | UpdatePostPayload {
	return {
		title: input.title,
		slug: input.slug,
		content: input.content,
		content_json: input.contentJson || undefined,
		excerpt: input.excerpt || undefined,
		featured_image_id: input.featuredImageId || undefined,
		status: input.status,
		visibility: input.visibility,
		is_sticky: input.isSticky,
		allow_comments: input.allowComments,
		meta_title: input.metaTitle || undefined,
		meta_description: input.metaDescription || undefined,
		canonical_url: input.canonicalUrl || undefined,
		og_image_url: input.ogImageUrl || undefined,
		category_ids: input.selectedCategoryIds.length > 0 ? input.selectedCategoryIds : undefined,
		tag_ids: input.selectedTagIds.length > 0 ? input.selectedTagIds : undefined,
		scheduled_at: input.status === 'scheduled' ? input.scheduledAt || undefined : undefined,
		post_password:
			input.visibility === 'password' ? input.postPassword || undefined : undefined,
		author_id: input.authorId || undefined,
		format: input.postFormat
	};
}

export function formatAutosaveRelative(lastSavedAt: Date | null): string {
	if (!lastSavedAt) return '';
	const seconds = Math.floor((Date.now() - lastSavedAt.getTime()) / 1000);
	if (seconds < 10) return 'just now';
	if (seconds < 60) return `${seconds}s ago`;
	return `${Math.floor(seconds / 60)} min ago`;
}
