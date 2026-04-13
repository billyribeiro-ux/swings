import { describe, expect, it } from 'vitest';
import { buildPostPayload, formatAutosaveRelative, slugifyPostTitle } from './post-editor-utils';

describe('post-editor-utils', () => {
	it('slugifies titles consistently', () => {
		expect(slugifyPostTitle('Hello, World! 2026')).toBe('hello-world-2026');
	});

	it('builds payload with optional fields omitted', () => {
		const payload = buildPostPayload({
			title: 'Title',
			slug: 'title',
			content: '<p>hello</p>',
			contentJson: null,
			excerpt: '',
			featuredImageId: undefined,
			status: 'draft',
			visibility: 'public',
			isSticky: false,
			allowComments: true,
			metaTitle: '',
			metaDescription: '',
			canonicalUrl: '',
			ogImageUrl: '',
			selectedCategoryIds: [],
			selectedTagIds: [],
			scheduledAt: '',
			postPassword: '',
			authorId: '',
			postFormat: 'standard'
		});

		expect(payload.title).toBe('Title');
		expect(payload.category_ids).toBeUndefined();
		expect(payload.post_password).toBeUndefined();
		expect(payload.scheduled_at).toBeUndefined();
	});

	it('formats autosave timestamp for recent saves', () => {
		const recentlySaved = new Date(Date.now() - 5_000);
		expect(formatAutosaveRelative(recentlySaved)).toBe('just now');
	});
});
