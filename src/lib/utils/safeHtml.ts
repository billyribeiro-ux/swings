/**
 * Client-/server-safe HTML sanitiser for `{@html ...}` escape hatches.
 *
 * WHY THIS EXISTS
 * The Rust backend sanitises author-supplied HTML at the write boundary (see
 * `backend/src/common/html.rs` → `sanitize_rich_text`). That's the
 * authoritative layer: any stored blog / course / form-block content was
 * already laundered through `ammonia` before it hit Postgres. The function
 * in this file is defence-in-depth so that:
 *
 *   1. Third-party / legacy content that predates the backend rule still
 *      gets cleaned on read.
 *   2. Any future feature that injects HTML *without* going through the
 *      backend (e.g. marketplace embeds, plugin output, cached responses
 *      from a misconfigured upstream) is still scrubbed before the browser
 *      executes it.
 *   3. We get a consistent surface: every `{@html untrusted}` site imports
 *      `safeHtml(...)` and grep greps clean.
 *
 * IMPLEMENTATION
 * `isomorphic-dompurify` wraps DOMPurify with a no-op-in-JSDOM bridge on
 * the server. That lets SvelteKit SSR render sanitised output without a
 * second code path — the browser hydration sees the same markup.
 *
 * POLICY
 * We inherit DOMPurify's defaults (block/inline tags, no `<script>`, no
 * `on*` handlers, no `javascript:` URLs) and additionally:
 *
 *   * Force `rel="noopener noreferrer"` on every surviving `<a>` via the
 *     `AFTER_SANITIZE_ATTRIBUTES` hook. DOMPurify normalises `target=_blank`
 *     but does not set rel; we do.
 *   * Drop `<form>` and form-inputs — we never want a rendered blog post
 *     to phish a logged-in reader.
 *
 * SCOPE
 * Returns a string. Callers drop it into `{@html ...}`. If you need a
 * plain-text version (email subjects, previews), use `safePlainText`.
 *
 * NEVER DO THIS
 *   <div>{@html untrusted}</div>
 * DO THIS
 *   <div>{@html safeHtml(untrusted)}</div>
 */

import DOMPurify from 'isomorphic-dompurify';

let hooksInstalled = false;

/**
 * DOMPurify treats hook registration as a process-global side effect.
 * Guard against installing the same hook twice on hot-reload / test
 * environments that re-import the module.
 */
function ensureHooks(): void {
	if (hooksInstalled) return;
	hooksInstalled = true;

	DOMPurify.addHook('afterSanitizeAttributes', (node) => {
		// Elements: force link safety. DOMPurify has already stripped
		// dangerous schemes and `target` values; we just harden `rel`.
		if ('tagName' in node && (node as Element).tagName === 'A') {
			const el = node as HTMLAnchorElement;
			el.setAttribute('rel', 'noopener noreferrer');
			// Open external links in a new tab. Internal `/...` refs can stay
			// in the same tab — preserves reading flow.
			const href = el.getAttribute('href') ?? '';
			if (href.startsWith('http')) el.setAttribute('target', '_blank');
		}
	});
}

const FORBIDDEN_TAGS = [
	// Form surfaces — never allow user HTML to host auth fields.
	'form',
	'input',
	'textarea',
	'select',
	'button',
	// Script / embed vectors (DOMPurify also blocks these by default; belt
	// and braces keeps grep honest).
	'script',
	'iframe',
	'object',
	'embed',
	'base',
	'meta'
];

const FORBIDDEN_ATTRS = [
	'style', // CSS injection / layout hijack
	'srcdoc', // iframe bypass vector
	'formaction', // form-submission hijack via <button>
	'target' // we re-add `target=_blank` in the hook for external links only
];

/**
 * Sanitise untrusted HTML for rendering via `{@html}`.
 *
 * Returns a safe HTML string. Safe to call with `null` / `undefined` —
 * produces an empty string, matching DOMPurify's null-safe contract.
 */
export function safeHtml(input: string | null | undefined): string {
	if (input == null || input === '') return '';
	ensureHooks();
	return DOMPurify.sanitize(input, {
		FORBID_TAGS: FORBIDDEN_TAGS,
		FORBID_ATTR: FORBIDDEN_ATTRS,
		KEEP_CONTENT: true,
		ALLOW_DATA_ATTR: false
	});
}

/**
 * Strip every tag and return the raw text. Useful for `<title>`, email
 * subjects, meta descriptions, or anywhere the consumer needs plain text
 * regardless of the authored format.
 */
export function safePlainText(input: string | null | undefined): string {
	if (input == null || input === '') return '';
	ensureHooks();
	const stripped = DOMPurify.sanitize(input, { ALLOWED_TAGS: [], KEEP_CONTENT: true });
	return stripped.replace(/\s+/g, ' ').trim();
}
