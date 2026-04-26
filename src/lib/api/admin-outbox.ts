/**
 * Phase 2.2 — admin client for `/api/admin/outbox/*`.
 */

import { api } from '$lib/api/client';
import type { components } from '$lib/api/schema';

export type OutboxRowDto = components['schemas']['OutboxRowDto'];
export type OutboxRetryResponse = components['schemas']['OutboxRetryResponse'];
export type PaginatedOutboxResponse =
	components['schemas']['PaginatedOutboxResponse'];

export type OutboxStatusFilter =
	| 'pending'
	| 'in_flight'
	| 'delivered'
	| 'failed'
	| 'dead_letter';

export interface OutboxListQuery {
	status?: OutboxStatusFilter | '';
	page?: number;
	per_page?: number;
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

export const outbox = {
	list(q: OutboxListQuery = {}): Promise<PaginatedOutboxResponse> {
		return api.get<PaginatedOutboxResponse>(
			`/api/admin/outbox${buildQs(q as Record<string, unknown>)}`
		);
	},
	get(id: string): Promise<OutboxRowDto> {
		return api.get<OutboxRowDto>(`/api/admin/outbox/${encodeURIComponent(id)}`);
	},
	retry(id: string): Promise<OutboxRetryResponse> {
		return api.post<OutboxRetryResponse>(
			`/api/admin/outbox/${encodeURIComponent(id)}/retry`
		);
	}
};
