/**
 * `safeHtml` — Vitest unit coverage.
 *
 * Scope:
 *   - Returns '' for null/undefined/'' input.
 *   - Strips <script> and inline event handlers.
 *   - Strips javascript: URLs from <a href>.
 *   - Forces rel="noopener noreferrer" on surviving links.
 *   - Forces target="_blank" on external (http) links; leaves relative alone.
 *   - Strips <form>/<input>/<iframe> per FORBIDDEN_TAGS.
 *   - Drops `style` attribute (CSS injection).
 *   - safePlainText strips all tags and collapses whitespace.
 */
import { describe, expect, it } from 'vitest';
import { safeHtml, safePlainText } from './safeHtml';

describe('safeHtml', () => {
	it('returns "" for null/undefined/""', () => {
		expect(safeHtml(null)).toBe('');
		expect(safeHtml(undefined)).toBe('');
		expect(safeHtml('')).toBe('');
	});

	it('strips <script> tags', () => {
		const out = safeHtml('<p>ok</p><script>alert("xss")</script>');
		expect(out).not.toContain('<script');
		expect(out).not.toContain('alert');
		expect(out).toContain('ok');
	});

	it('strips inline on* handlers', () => {
		const out = safeHtml('<a href="/x" onclick="alert(1)">link</a>');
		expect(out).not.toMatch(/onclick/i);
		expect(out).toContain('link');
	});

	it('strips javascript: URLs from anchor href', () => {
		 
		const out = safeHtml('<a href="javascript:alert(1)">click</a>');
		expect(out).not.toMatch(/javascript:/i);
	});

	it('forces rel="noopener noreferrer" on surviving anchors', () => {
		const out = safeHtml('<a href="https://example.com">go</a>');
		expect(out).toMatch(/rel="noopener noreferrer"/);
	});

	it('opens external links in a new tab via target="_blank"', () => {
		const out = safeHtml('<a href="https://example.com">go</a>');
		expect(out).toMatch(/target="_blank"/);
	});

	it('leaves relative links without target=_blank', () => {
		const out = safeHtml('<a href="/about">about</a>');
		expect(out).not.toMatch(/target=/);
	});

	it('strips <form> and form-input tags but keeps inner text', () => {
		const out = safeHtml(
			'<form><input name="user"><button>send</button>label</form>'
		);
		expect(out).not.toContain('<form');
		expect(out).not.toContain('<input');
		expect(out).not.toContain('<button');
		// `KEEP_CONTENT: true` retains inner text.
		expect(out).toContain('label');
	});

	it('strips <iframe>', () => {
		const out = safeHtml('<iframe src="https://evil.test"></iframe>safe');
		expect(out).not.toContain('<iframe');
		expect(out).toContain('safe');
	});

	it('drops style attribute', () => {
		const out = safeHtml('<p style="color:red">hi</p>');
		expect(out).not.toMatch(/style=/);
		expect(out).toContain('hi');
	});

	it('preserves common formatting tags', () => {
		const out = safeHtml('<p><strong>bold</strong> and <em>italic</em></p>');
		expect(out).toContain('<strong>');
		expect(out).toContain('<em>');
	});
});

describe('safePlainText', () => {
	it('returns "" for nullish input', () => {
		expect(safePlainText(null)).toBe('');
		expect(safePlainText(undefined)).toBe('');
		expect(safePlainText('')).toBe('');
	});

	it('strips every tag and collapses whitespace', () => {
		const out = safePlainText('<h1>Hello</h1>\n\n<p>  world  </p>');
		expect(out).toBe('Hello world');
	});

	it('strips script content entirely', () => {
		const out = safePlainText('safe<script>alert(1)</script>');
		expect(out).toBe('safe');
	});
});
