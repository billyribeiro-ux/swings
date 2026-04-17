/**
 * FORM-10: Public form route — SSR-loaded definition.
 *
 * Hits `GET /api/forms/{slug}` (skipAuth — public endpoint) and surfaces the
 * shape `FormRenderer` expects. Uses SvelteKit's fetch so the server-side
 * pass primes the cache and the browser's first paint carries the schema.
 */
import { getPublicApiBase } from '$lib/api/publicApiBase';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { FormDefinition } from '$lib/api/forms';

const API = getPublicApiBase();

export const load: PageServerLoad = async ({ params, fetch }) => {
	const res = await fetch(`${API}/api/forms/${encodeURIComponent(params.slug)}`);

	if (res.status === 404) {
		error(404, 'Form not found');
	}
	if (!res.ok) {
		error(500, 'Failed to load form');
	}

	const definition: FormDefinition = await res.json();
	return { definition };
};
