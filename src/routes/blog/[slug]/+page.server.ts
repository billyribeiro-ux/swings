import { getPublicApiBase } from '$lib/api/publicApiBase';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { BlogPostResponse } from '$lib/api/types';

const API = getPublicApiBase();

export const load: PageServerLoad = async ({ params, fetch }) => {
	const res = await fetch(`${API}/api/blog/posts/${params.slug}`);

	if (res.status === 404) {
		error(404, 'Post not found');
	}

	if (!res.ok) {
		error(500, 'Failed to load post');
	}

	const post: BlogPostResponse = await res.json();

	return { post };
};
