/**
 * Phase 2.4 — admin client for `/api/admin/forms/{id}/submissions`.
 */

import { api } from '$lib/api/client';
import type { components } from '$lib/api/schema';

export type SubmissionRow = components['schemas']['SubmissionRow'];
export type PaginatedSubmissions = components['schemas']['PaginatedSubmissions'];

export interface SubmissionListQuery {
	status?: string;
	from?: string;
	to?: string;
	page?: number;
	per_page?: number;
}

export type BulkAction = 'mark_spam' | 'restore' | 'delete';

export interface BulkActionRequest {
	ids: string[];
	action: BulkAction;
}

export interface BulkActionResponse {
	updated: number;
}

function buildQs(params: Record<string, unknown>): string {
	const u = new URLSearchParams();
	for (const [k, v] of Object.entries(params)) {
		if (v === undefined || v === null || v === '') continue;
		u.set(k, String(v));
	}
	const s = u.toString();
	return s ? `?${s}` : '';
}

export const formSubmissions = {
	list(formId: string, q: SubmissionListQuery = {}): Promise<PaginatedSubmissions> {
		return api.get<PaginatedSubmissions>(
			`/api/admin/forms/${encodeURIComponent(formId)}/submissions${buildQs(
				q as Record<string, unknown>
			)}`
		);
	},
	bulkAction(formId: string, body: BulkActionRequest): Promise<BulkActionResponse> {
		return api.post<BulkActionResponse>(
			`/api/admin/forms/${encodeURIComponent(formId)}/submissions/bulk`,
			body
		);
	},
	csvExportUrl(formId: string, q: SubmissionListQuery = {}): string {
		return `/api/admin/forms/${encodeURIComponent(formId)}/submissions${buildQs({
			...q,
			format: 'csv'
		} as Record<string, unknown>)}`;
	}
};
