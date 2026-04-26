import { redirect } from '@sveltejs/kit';

/**
 * Phase 2.3 — `/admin/consent/policies` (plural) is preserved as a
 * permanent alias of the canonical `/admin/consent/policy` route. The
 * previous stub used a different design system and predated the spec
 * naming convention; redirecting here keeps any bookmarked URLs alive
 * while routing operators to the production-grade page.
 */
export function load() {
	throw redirect(308, '/admin/consent/policy');
}
