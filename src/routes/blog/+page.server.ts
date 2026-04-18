import { getPublicApiBase } from '$lib/api/publicApiBase';
import type { PageServerLoad } from './$types';
import type { BlogPostListItem, BlogCategory, PaginatedResponse } from '$lib/api/types';

const API = getPublicApiBase();

export const load: PageServerLoad = async ({ url, fetch }) => {
	// `Number()` never throws but returns NaN for non-numeric input; guard so the
	// API call always receives a valid positive integer page number.
	const rawPage = Number(url.searchParams.get('page') ?? '1');
	const page = Number.isFinite(rawPage) && rawPage >= 1 ? Math.floor(rawPage) : 1;
	const per_page = 12;

	const [postsRes, catsRes] = await Promise.allSettled([
		fetch(`${API}/api/blog/posts?page=${page}&per_page=${per_page}`),
		fetch(`${API}/api/blog/categories`)
	]);

	let posts: BlogPostListItem[] = [];
	let total = 0;
	let totalPages = 1;
	let categories: BlogCategory[] = [];

	if (postsRes.status === 'fulfilled' && postsRes.value.ok) {
		const data: PaginatedResponse<BlogPostListItem> = await postsRes.value.json();
		posts = data.data;
		total = data.total;
		totalPages = data.total_pages;
	}

	if (catsRes.status === 'fulfilled' && catsRes.value.ok) {
		categories = await catsRes.value.json();
	}

	return { posts, categories, total, totalPages, page };
};
