/**
 * FORM-02: Client-side validation engine.
 *
 * Byte-for-byte-compatible port of `backend/src/forms/validation.rs`. Both
 * sides return the same error codes for the same inputs so a server rejection
 * never tells the user something the client didn't also catch (and vice versa).
 *
 * The field/schema types here mirror the Rust `FieldSchema` enum's wire
 * representation (`{ "type": "email", "key": "email", "required": true, ... }`).
 * Once the OpenAPI codegen includes these shapes the duplicated types collapse
 * into aliases — until then the definitions are kept in sync by hand.
 *
 * Async rules: unlike the Rust side, the client returns synchronously and
 * delegates async checks (e.g. `unique_email`) to an optional callback. The
 * default path is sync-only because the typical use-case is optimistic UX.
 */

// ── Error + result types ──────────────────────────────────────────────

export interface ValidationError {
	readonly field_key: string;
	readonly code: string;
	readonly message: string;
}

export type FormData = Readonly<Record<string, unknown>>;

export type AsyncRuleName = 'unique_email';

export interface AsyncRuleRunner {
	run(
		fieldKey: string,
		rule: AsyncRuleName,
		value: unknown
	): Promise<ValidationError | null>;
}

export const noopAsyncRuleRunner: AsyncRuleRunner = {
	async run() {
		return null;
	}
};

// ── Field schema (wire) ───────────────────────────────────────────────

interface FieldMeta {
	readonly key: string;
	readonly label?: string;
	readonly placeholder?: string;
	readonly helpText?: string;
	readonly required?: boolean;
}

interface LengthRules {
	readonly min_length?: number;
	readonly max_length?: number;
}

interface NumberRules {
	readonly min?: number;
	readonly max?: number;
	readonly step?: number;
}

interface FileRules {
	readonly min_files?: number;
	readonly max_files?: number;
	readonly allowed_mime_types?: readonly string[];
	readonly max_file_size?: number;
}

/**
 * Discriminated union over the 33 field types. Omitted variants use an
 * index signature fall-through so unknown server-side types don't crash
 * the client; they simply skip validation beyond `required`.
 */
export type FieldSchema =
	| ({ type: 'text'; pattern?: string } & FieldMeta & LengthRules)
	| ({ type: 'email'; async_rule?: AsyncRuleName } & FieldMeta)
	| ({ type: 'phone' } & FieldMeta)
	| ({ type: 'url' } & FieldMeta)
	| ({ type: 'textarea' } & FieldMeta & LengthRules)
	| ({ type: 'number' } & FieldMeta & NumberRules)
	| ({ type: 'slider'; min: number; max: number; step?: number } & FieldMeta)
	| ({ type: 'rating'; max_stars?: number } & FieldMeta)
	| ({ type: 'date'; min_date?: string; max_date?: string } & FieldMeta)
	| ({ type: 'time' } & FieldMeta)
	| ({ type: 'datetime' } & FieldMeta)
	| ({ type: 'select'; options: readonly ChoiceOption[] } & FieldMeta)
	| ({
			type: 'multi_select';
			options: readonly ChoiceOption[];
			min_selections?: number;
			max_selections?: number;
	  } & FieldMeta)
	| ({ type: 'radio'; options: readonly ChoiceOption[] } & FieldMeta)
	| ({ type: 'checkbox' } & FieldMeta)
	| ({ type: 'file_upload' } & FieldMeta & FileRules)
	| ({ type: 'image_upload' } & FieldMeta & FileRules)
	| ({ type: 'signature' } & FieldMeta)
	| ({ type: 'rich_text' } & FieldMeta & LengthRules)
	| ({ type: 'hidden'; default_value?: unknown } & FieldMeta)
	| ({ type: 'html_block'; html: string } & FieldMeta)
	| ({ type: 'section_break' } & FieldMeta)
	| ({ type: 'page_break' } & FieldMeta)
	| ({ type: 'address' } & FieldMeta)
	| ({ type: 'gdpr_consent'; consent_text: string } & FieldMeta)
	| ({ type: 'terms'; terms_url: string } & FieldMeta)
	| ({ type: 'custom_html'; html: string } & FieldMeta)
	| ({ type: 'payment'; amount_cents: number; currency?: string } & FieldMeta)
	| ({ type: 'subscription'; plan_id: string } & FieldMeta)
	| ({
			type: 'quiz';
			question: string;
			options: readonly ChoiceOption[];
			correct_value: string;
			points?: number;
	  } & FieldMeta)
	| ({ type: 'nps' } & FieldMeta)
	| ({ type: 'likert'; scale?: number } & FieldMeta)
	| ({ type: 'matrix'; rows: readonly string[]; columns: readonly string[] } & FieldMeta)
	| ({ type: 'ranking'; options: readonly ChoiceOption[] } & FieldMeta)
	| ({ type: 'calculation'; formula: string } & FieldMeta)
	| ({ type: 'dynamic_dropdown'; endpoint: string } & FieldMeta)
	| ({ type: 'country_state' } & FieldMeta)
	| ({ type: 'post_product_selector'; source: string } & FieldMeta);

export interface ChoiceOption {
	readonly value: string;
	readonly label: string;
	readonly disabled?: boolean;
}

// ── Regex constants (mirror Rust) ─────────────────────────────────────

const EMAIL_RE = /^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$/;
const PHONE_RE = /^\+[1-9][0-9]{6,14}$/;

// ── Entry point ───────────────────────────────────────────────────────

/**
 * Validate a form's data against its schema. The default runner is a no-op;
 * pass a real [[AsyncRuleRunner]] when you need cross-field DB checks (e.g.
 * `unique_email`) — the rest of the rules run synchronously.
 */
export async function validate(
	schema: readonly FieldSchema[],
	data: FormData,
	runner: AsyncRuleRunner = noopAsyncRuleRunner
): Promise<ValidationError[]> {
	const errors: ValidationError[] = [];

	for (const field of schema) {
		if (isDecorative(field)) continue;
		validateField(field, data, errors);
	}

	// Async pass — only the `email.async_rule` path for now.
	for (const field of schema) {
		if (field.type === 'email' && field.async_rule) {
			const value = data[field.key];
			if (!isEmpty(value)) {
				const err = await runner.run(field.key, field.async_rule, value);
				if (err) errors.push(err);
			}
		}
	}

	return errors;
}

// ── Per-field dispatch ────────────────────────────────────────────────

function validateField(
	field: FieldSchema,
	data: FormData,
	errors: ValidationError[]
): void {
	const value = data[field.key];

	if (field.required && isEmpty(value)) {
		errors.push(err(field.key, 'required', `${label(field)} is required.`));
		return;
	}
	if (isEmpty(value)) return;

	switch (field.type) {
		case 'text':
			checkString(field, value, errors);
			if (field.pattern) checkPattern(field, value, field.pattern, errors);
			break;
		case 'textarea':
		case 'rich_text':
			checkString(field, value, errors);
			break;
		case 'email':
			checkEmail(field, value, errors);
			break;
		case 'phone':
			checkPhone(field, value, errors);
			break;
		case 'url':
			checkUrl(field, value, errors);
			break;
		case 'number':
			checkNumber(field, value, field, errors);
			break;
		case 'slider':
			checkNumber(field, value, { min: field.min, max: field.max }, errors);
			break;
		case 'rating':
			checkRatingRange(field, value, 1, field.max_stars ?? 5, 'rating_range', errors);
			break;
		case 'nps':
			checkRatingRange(field, value, 0, 10, 'nps_range', errors);
			break;
		case 'likert':
			checkRatingRange(field, value, 1, field.scale ?? 5, 'likert_range', errors);
			break;
		case 'date':
			checkDate(field, value, field.min_date, field.max_date, errors);
			break;
		case 'time':
			checkTime(field, value, errors);
			break;
		case 'datetime':
			checkDatetime(field, value, errors);
			break;
		case 'file_upload':
		case 'image_upload':
			checkFiles(field, value, field, errors);
			break;
		case 'multi_select':
			checkMultiSelect(field, value, field.min_selections, field.max_selections, errors);
			break;
		default:
			break;
	}

	// Cross-field equals: `<other>_confirm` must equal `<other>`.
	if (field.key.endsWith('_confirm')) {
		const stem = field.key.slice(0, -'_confirm'.length);
		if (stem in data) {
			const other = data[stem];
			if (!deepEqual(other, value)) {
				errors.push(err(field.key, 'equals', `${label(field)} does not match.`));
			}
		}
	}
}

// ── Helpers ───────────────────────────────────────────────────────────

function err(
	field_key: string,
	code: string,
	message: string
): ValidationError {
	return { field_key, code, message };
}

function label(f: FieldSchema): string {
	return f.label && f.label.length > 0 ? f.label : f.key;
}

function isEmpty(value: unknown): boolean {
	if (value === null || value === undefined) return true;
	if (typeof value === 'string') return value.trim().length === 0;
	if (Array.isArray(value)) return value.length === 0;
	if (typeof value === 'object') return Object.keys(value).length === 0;
	return false;
}

function isDecorative(f: FieldSchema): boolean {
	return (
		f.type === 'html_block' ||
		f.type === 'section_break' ||
		f.type === 'page_break' ||
		f.type === 'custom_html'
	);
}

function checkString(
	field: FieldSchema,
	value: unknown,
	errors: ValidationError[]
): void {
	if (typeof value !== 'string') {
		errors.push(err(field.key, 'type', `${label(field)} must be text.`));
		return;
	}
	const len = Array.from(value).length;
	const rules = field as Partial<LengthRules>;
	if (rules.min_length !== undefined && len < rules.min_length) {
		errors.push(
			err(field.key, 'min_length', `${label(field)} must be at least ${rules.min_length} characters.`)
		);
	}
	if (rules.max_length !== undefined && len > rules.max_length) {
		errors.push(
			err(field.key, 'max_length', `${label(field)} must be at most ${rules.max_length} characters.`)
		);
	}
}

function checkPattern(
	field: FieldSchema,
	value: unknown,
	pattern: string,
	errors: ValidationError[]
): void {
	if (typeof value !== 'string') return;
	let re: RegExp;
	try {
		re = new RegExp(pattern);
	} catch {
		// Bad schema regex — skip silently; admin bug, not user bug.
		return;
	}
	if (!re.test(value)) {
		errors.push(err(field.key, 'pattern', `${label(field)} has an invalid format.`));
	}
}

function checkEmail(
	field: FieldSchema,
	value: unknown,
	errors: ValidationError[]
): void {
	const s = typeof value === 'string' ? value : '';
	if (!EMAIL_RE.test(s)) {
		errors.push(
			err(field.key, 'email', `${label(field)} must be a valid email address.`)
		);
	}
}

function checkPhone(
	field: FieldSchema,
	value: unknown,
	errors: ValidationError[]
): void {
	const s = typeof value === 'string' ? value : '';
	if (!PHONE_RE.test(s)) {
		errors.push(
			err(
				field.key,
				'phone',
				`${label(field)} must be an E.164 phone number (e.g. +14155551234).`
			)
		);
	}
}

function checkUrl(
	field: FieldSchema,
	value: unknown,
	errors: ValidationError[]
): void {
	const s = typeof value === 'string' ? value : '';
	let ok: boolean;
	try {
		new URL(s);
		ok = true;
	} catch {
		ok = false;
	}
	if (!ok) {
		errors.push(err(field.key, 'url', `${label(field)} must be a valid URL.`));
	}
}

function checkNumber(
	field: FieldSchema,
	value: unknown,
	rules: { min?: number; max?: number },
	errors: ValidationError[]
): void {
	const n = toFiniteNumber(value);
	if (n === null) {
		errors.push(err(field.key, 'type', `${label(field)} must be a number.`));
		return;
	}
	if (rules.min !== undefined && n < rules.min) {
		errors.push(err(field.key, 'min', `${label(field)} must be at least ${rules.min}.`));
	}
	if (rules.max !== undefined && n > rules.max) {
		errors.push(err(field.key, 'max', `${label(field)} must be at most ${rules.max}.`));
	}
}

function checkRatingRange(
	field: FieldSchema,
	value: unknown,
	min: number,
	max: number,
	code: string,
	errors: ValidationError[]
): void {
	const n = toFiniteNumber(value);
	if (n === null || !Number.isInteger(n)) {
		errors.push(err(field.key, 'type', `${label(field)} must be a number.`));
		return;
	}
	if (n < min || n > max) {
		errors.push(err(field.key, code, `${label(field)} must be between ${min} and ${max}.`));
	}
}

function checkDate(
	field: FieldSchema,
	value: unknown,
	min: string | undefined,
	max: string | undefined,
	errors: ValidationError[]
): void {
	const s = typeof value === 'string' ? value : '';
	const parsed = parseIsoDate(s);
	if (!parsed) {
		errors.push(err(field.key, 'date', `${label(field)} must be an ISO-8601 date (YYYY-MM-DD).`));
		return;
	}
	if (min) {
		const md = parseIsoDate(min);
		if (md && parsed < md) {
			errors.push(err(field.key, 'min_date', `${label(field)} must be on or after ${min}.`));
		}
	}
	if (max) {
		const md = parseIsoDate(max);
		if (md && parsed > md) {
			errors.push(err(field.key, 'max_date', `${label(field)} must be on or before ${max}.`));
		}
	}
}

function checkTime(
	field: FieldSchema,
	value: unknown,
	errors: ValidationError[]
): void {
	const s = typeof value === 'string' ? value : '';
	const ok = /^\d{2}:\d{2}(?::\d{2})?$/.test(s);
	if (!ok) {
		errors.push(err(field.key, 'time', `${label(field)} must be a 24-hour time (HH:MM or HH:MM:SS).`));
	}
}

function checkDatetime(
	field: FieldSchema,
	value: unknown,
	errors: ValidationError[]
): void {
	const s = typeof value === 'string' ? value : '';
	const ok = s.length > 0 && !Number.isNaN(Date.parse(s));
	if (!ok) {
		errors.push(err(field.key, 'datetime', `${label(field)} must be an RFC 3339 datetime.`));
	}
}

interface FileDescriptor {
	readonly mime_type?: string;
	readonly size?: number;
}

function checkFiles(
	field: FieldSchema,
	value: unknown,
	rules: FileRules,
	errors: ValidationError[]
): void {
	if (!Array.isArray(value)) {
		errors.push(err(field.key, 'type', `${label(field)} must be a list of files.`));
		return;
	}
	const arr = value as FileDescriptor[];
	if (rules.min_files !== undefined && arr.length < rules.min_files) {
		errors.push(
			err(field.key, 'min_files', `${label(field)} requires at least ${rules.min_files} file(s).`)
		);
	}
	if (rules.max_files !== undefined && arr.length > rules.max_files) {
		errors.push(
			err(field.key, 'max_files', `${label(field)} accepts at most ${rules.max_files} file(s).`)
		);
	}
	arr.forEach((f, idx) => {
		if (rules.max_file_size !== undefined && typeof f?.size === 'number' && f.size > rules.max_file_size) {
			errors.push(
				err(
					field.key,
					'max_file_size',
					`File #${idx + 1} on ${label(field)} exceeds ${rules.max_file_size} byte limit.`
				)
			);
		}
		if (rules.allowed_mime_types && rules.allowed_mime_types.length > 0) {
			const mime = typeof f?.mime_type === 'string' ? f.mime_type : '';
			const ok = rules.allowed_mime_types.some(
				(m) => m.toLowerCase() === mime.toLowerCase()
			);
			if (!ok) {
				errors.push(
					err(
						field.key,
						'mime_type',
						`File #${idx + 1} on ${label(field)} has an unsupported type.`
					)
				);
			}
		}
	});
}

function checkMultiSelect(
	field: FieldSchema,
	value: unknown,
	min: number | undefined,
	max: number | undefined,
	errors: ValidationError[]
): void {
	if (!Array.isArray(value)) {
		errors.push(err(field.key, 'type', `${label(field)} must be a list.`));
		return;
	}
	if (min !== undefined && value.length < min) {
		errors.push(
			err(field.key, 'min_selections', `${label(field)} requires at least ${min} selection(s).`)
		);
	}
	if (max !== undefined && value.length > max) {
		errors.push(
			err(field.key, 'max_selections', `${label(field)} accepts at most ${max} selection(s).`)
		);
	}
}

function toFiniteNumber(value: unknown): number | null {
	if (typeof value === 'number' && Number.isFinite(value)) return value;
	if (typeof value === 'string' && value.trim().length > 0) {
		const n = Number(value);
		if (Number.isFinite(n)) return n;
	}
	return null;
}

function parseIsoDate(s: string): number | null {
	const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
	if (!m) return null;
	const year = Number(m[1]);
	const month = Number(m[2]);
	const day = Number(m[3]);
	if (month < 1 || month > 12 || day < 1 || day > 31) return null;
	// UTC timestamp; comparable as a number.
	return Date.UTC(year, month - 1, day);
}

function deepEqual(a: unknown, b: unknown): boolean {
	if (a === b) return true;
	if (typeof a !== typeof b) return false;
	if (a === null || b === null) return false;
	if (Array.isArray(a) && Array.isArray(b)) {
		if (a.length !== b.length) return false;
		return a.every((el, i) => deepEqual(el, b[i]));
	}
	if (typeof a === 'object' && typeof b === 'object') {
		const ao = a as Record<string, unknown>;
		const bo = b as Record<string, unknown>;
		const ak = Object.keys(ao);
		const bk = Object.keys(bo);
		if (ak.length !== bk.length) return false;
		return ak.every((k) => deepEqual(ao[k], bo[k]));
	}
	return false;
}
