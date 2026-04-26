import { describe, it, expect } from 'vitest';
import { validate, type FieldSchema, type AsyncRuleRunner } from './validate';

describe('forms/validate', () => {
	it('flags missing required values', async () => {
		const schema: FieldSchema[] = [{ type: 'text', key: 'name', required: true }];
		const errs = await validate(schema, {});
		expect(errs).toHaveLength(1);
		expect(errs[0]!.code).toBe('required');
		expect(errs[0]!.field_key).toBe('name');
	});

	it('treats whitespace-only strings as empty', async () => {
		const schema: FieldSchema[] = [{ type: 'text', key: 'name', required: true }];
		const errs = await validate(schema, { name: '   ' });
		expect(errs[0]!.code).toBe('required');
	});

	it('emits min_length / max_length codes', async () => {
		const schema: FieldSchema[] = [{ type: 'text', key: 'name', min_length: 3, max_length: 5 }];
		const short = await validate(schema, { name: 'ab' });
		expect(short.some((e) => e.code === 'min_length')).toBe(true);
		const long = await validate(schema, { name: 'abcdefg' });
		expect(long.some((e) => e.code === 'max_length')).toBe(true);
		const ok = await validate(schema, { name: 'abcd' });
		expect(ok).toHaveLength(0);
	});

	it('emits numeric min/max codes', async () => {
		const schema: FieldSchema[] = [{ type: 'number', key: 'age', min: 18, max: 65 }];
		const lo = await validate(schema, { age: 10 });
		expect(lo.some((e) => e.code === 'min')).toBe(true);
		const hi = await validate(schema, { age: 80 });
		expect(hi.some((e) => e.code === 'max')).toBe(true);
	});

	it('rejects unmatched regex pattern', async () => {
		const schema: FieldSchema[] = [{ type: 'text', key: 'code', pattern: '^[A-Z]{3}$' }];
		const bad = await validate(schema, { code: 'abc' });
		expect(bad.some((e) => e.code === 'pattern')).toBe(true);
		const ok = await validate(schema, { code: 'ABC' });
		expect(ok).toHaveLength(0);
	});

	it('validates email format', async () => {
		const schema: FieldSchema[] = [{ type: 'email', key: 'email' }];
		const ok = await validate(schema, { email: 'jane@example.com' });
		expect(ok).toHaveLength(0);
		const bad = await validate(schema, { email: 'not-an-email' });
		expect(bad.some((e) => e.code === 'email')).toBe(true);
	});

	it('validates url and phone', async () => {
		const schema: FieldSchema[] = [
			{ type: 'url', key: 'site' },
			{ type: 'phone', key: 'tel' }
		];
		const errs = await validate(schema, { site: 'not a url', tel: '555-1234' });
		expect(errs.some((e) => e.code === 'url')).toBe(true);
		expect(errs.some((e) => e.code === 'phone')).toBe(true);
		const ok = await validate(schema, {
			site: 'https://example.com/',
			tel: '+14155551234'
		});
		expect(ok).toHaveLength(0);
	});

	it('validates date / time / datetime', async () => {
		const schema: FieldSchema[] = [
			{ type: 'date', key: 'd' },
			{ type: 'time', key: 't' },
			{ type: 'datetime', key: 'dt' }
		];
		const bad = await validate(schema, { d: 'foo', t: 'bar', dt: 'baz' });
		expect(bad.some((e) => e.code === 'date')).toBe(true);
		expect(bad.some((e) => e.code === 'time')).toBe(true);
		expect(bad.some((e) => e.code === 'datetime')).toBe(true);
		const ok = await validate(schema, {
			d: '2026-04-17',
			t: '14:30',
			dt: '2026-04-17T14:30:00Z'
		});
		expect(ok).toHaveLength(0);
	});

	it('validates file rules (min/max/size/mime)', async () => {
		const schema: FieldSchema[] = [
			{
				type: 'file_upload',
				key: 'cv',
				min_files: 1,
				max_files: 2,
				allowed_mime_types: ['application/pdf'],
				max_file_size: 1024
			}
		];
		const tooBig = await validate(schema, {
			cv: [{ mime_type: 'application/pdf', size: 2048 }]
		});
		expect(tooBig.some((e) => e.code === 'max_file_size')).toBe(true);
		const badMime = await validate(schema, {
			cv: [{ mime_type: 'image/png', size: 100 }]
		});
		expect(badMime.some((e) => e.code === 'mime_type')).toBe(true);
		const ok = await validate(schema, {
			cv: [{ mime_type: 'application/pdf', size: 100 }]
		});
		expect(ok).toHaveLength(0);
	});

	it('validates rating and NPS ranges', async () => {
		const schema: FieldSchema[] = [
			{ type: 'rating', key: 'score', max_stars: 5 },
			{ type: 'nps', key: 'nps' }
		];
		const errs = await validate(schema, { score: 7, nps: 15 });
		expect(errs.some((e) => e.code === 'rating_range')).toBe(true);
		expect(errs.some((e) => e.code === 'nps_range')).toBe(true);
	});

	it('enforces cross-field _confirm equality', async () => {
		const schema: FieldSchema[] = [
			{ type: 'text', key: 'password', required: true },
			{ type: 'text', key: 'password_confirm', required: true }
		];
		const mismatch = await validate(schema, {
			password: 'secret',
			password_confirm: 'typo'
		});
		expect(mismatch.some((e) => e.code === 'equals')).toBe(true);
		const ok = await validate(schema, {
			password: 'secret',
			password_confirm: 'secret'
		});
		expect(ok).toHaveLength(0);
	});

	it('invokes the async rule runner for email.async_rule', async () => {
		const schema: FieldSchema[] = [{ type: 'email', key: 'email', async_rule: 'unique_email' }];
		const runner: AsyncRuleRunner = {
			async run(fieldKey) {
				return {
					field_key: fieldKey,
					code: 'unique_email',
					message: 'This email is already registered.'
				};
			}
		};
		const errs = await validate(schema, { email: 'jane@example.com' }, runner);
		expect(errs.some((e) => e.code === 'unique_email')).toBe(true);
	});
});
