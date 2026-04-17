/**
 * Forms API client.
 *
 * Thin wrapper around the generated OpenAPI types. The `FormDefinition`,
 * `SubmitRequest`, `SubmitResponse`, `PartialRequest`, and `PartialResponse`
 * aliases point at `components['schemas'][...]` so adding a field on the
 * backend automatically flows through here without manual sync.
 *
 * File uploads (FORM-05) are accepted as an `opts.files` parameter on
 * `submitForm` but not yet processed — the real multipart pipeline lands
 * with that subsystem.
 */

import { api } from '$lib/api/client';
import type { components } from '$lib/api/schema';

export type FormDefinition = components['schemas']['FormDefinition'];
export type SubmitRequest = components['schemas']['SubmitRequest'];
export type SubmitResponse = components['schemas']['SubmitResponse'];
export type PartialRequest = components['schemas']['PartialRequest'];
export type PartialResponse = components['schemas']['PartialResponse'];
export type FormValidationError = components['schemas']['ValidationError'];

export type FormData = Readonly<Record<string, unknown>>;

export interface SubmitOptions {
	readonly utm?: Readonly<Record<string, string>>;
	readonly anonymousId?: string;
	/** Pre-uploaded file descriptors (FORM-05). */
	readonly files?: readonly FileDescriptor[];
}

interface FileDescriptor {
	readonly field_key: string;
	readonly file_id: string;
	readonly filename: string;
	readonly size: number;
	readonly sha256: string;
	readonly mime_type: string;
}

/**
 * Fetch the active form definition (schema + logic + settings) by slug.
 * Throws `ApiError` on 404 / 5xx.
 */
export async function fetchFormSchema(slug: string): Promise<FormDefinition> {
	return api.get<FormDefinition>(`/api/forms/${encodeURIComponent(slug)}`, {
		skipAuth: true
	});
}

/**
 * Submit a form. On server-side validation failure the response is a 422
 * with `{ errors: ValidationError[] }` — the `ApiClient` surfaces it as an
 * `ApiError` whose message is the Problem `detail`. Call sites that need the
 * per-field array should catch and re-read the response; this helper is
 * intentionally thin.
 */
export async function submitForm(
	slug: string,
	data: FormData,
	opts: SubmitOptions = {}
): Promise<SubmitResponse> {
	const body: SubmitRequest = {
		data: data as unknown as SubmitRequest['data'],
		utm: (opts.utm ?? {}) as unknown as SubmitRequest['utm'],
		files: (opts.files ?? []) as unknown as SubmitRequest['files'],
		anonymous_id: opts.anonymousId ?? null
	} as SubmitRequest;
	return api.post<SubmitResponse>(
		`/api/forms/${encodeURIComponent(slug)}/submit`,
		body,
		{ skipAuth: true }
	);
}

/**
 * Save a partial draft. Returns a fresh resume token the caller can embed
 * in an email link.
 */
export async function savePartial(
	slug: string,
	data: FormData,
	resumeToken?: string
): Promise<PartialResponse> {
	const body: PartialRequest = {
		data: data as unknown as PartialRequest['data'],
		resume_token: resumeToken ?? null
	} as PartialRequest;
	return api.post<PartialResponse>(
		`/api/forms/${encodeURIComponent(slug)}/partial`,
		body,
		{ skipAuth: true }
	);
}

/**
 * Fetch a previously-saved partial by token.
 */
export async function loadPartial(
	slug: string,
	resumeToken: string
): Promise<FormData> {
	return api.get<FormData>(
		`/api/forms/${encodeURIComponent(slug)}/partial?token=${encodeURIComponent(resumeToken)}`,
		{ skipAuth: true }
	);
}
