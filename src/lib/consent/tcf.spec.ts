/**
 * CONSENT-04 — TCF v2.2 publisher-only shim unit tests.
 *
 * We only pin the pure string-build step (category map → TC string payload).
 * The `__tcfapi` listener plumbing is DOM-dependent and covered by the
 * browser spec.
 */
import { describe, expect, it } from 'vitest';
import { buildTcString } from './tcf';

function decodePayload(tcString: string): Record<string, unknown> {
	const parts = tcString.split('.');
	expect(parts).toHaveLength(3);
	expect(parts[0]).toBe('swings-pub-v1');
	const b64 = parts[1]!;
	// base64url → base64
	const normal = b64.replace(/-/g, '+').replace(/_/g, '/');
	const padded = normal + '==='.slice((normal.length + 3) % 4);
	const binary = atob(padded);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	const json = new TextDecoder().decode(bytes);
	return JSON.parse(json) as Record<string, unknown>;
}

describe('buildTcString', () => {
	it('returns a parseable 3-part string with version prefix', () => {
		const s = buildTcString({ necessary: true });
		const parts = s.split('.');
		expect(parts).toHaveLength(3);
		expect(parts[0]).toBe('swings-pub-v1');
		// Third part is a base-36 encoded epoch-second — positive integer.
		expect(parseInt(parts[2]!, 36)).toBeGreaterThan(0);
	});

	it('encodes analytics as purposes 7, 8, 10 when granted', () => {
		const s = buildTcString({ analytics: true });
		const payload = decodePayload(s);
		expect(payload.p).toEqual([7, 8, 10]);
	});

	it('encodes marketing as purposes 2, 3, 4, 7, 9 when granted', () => {
		const s = buildTcString({ marketing: true });
		const payload = decodePayload(s);
		expect(payload.p).toEqual([2, 3, 4, 7, 9]);
	});

	it('deduplicates overlapping purposes (analytics + marketing share 7)', () => {
		const s = buildTcString({ analytics: true, marketing: true });
		const payload = decodePayload(s);
		// 2,3,4 (marketing), 7 (shared), 8,10 (analytics), 9 (marketing) — unique + sorted.
		expect(payload.p).toEqual([2, 3, 4, 7, 8, 9, 10]);
	});

	it('emits empty purposes array when every non-necessary category is denied', () => {
		const s = buildTcString({
			necessary: true,
			analytics: false,
			marketing: false,
			functional: false,
			personalization: false
		});
		const payload = decodePayload(s);
		expect(payload.p).toEqual([]);
	});

	it('carries tcfPolicyVersion 5 and the reserved cmp_id', () => {
		const s = buildTcString({ analytics: true });
		const payload = decodePayload(s);
		expect(payload.tcf_policy_version).toBe(5);
		expect(payload.cmp_id).toBe(0xffff);
	});
});
