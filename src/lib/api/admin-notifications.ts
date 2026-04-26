/**
 * Phase 2.1 — admin client for `/api/admin/notifications/*`.
 *
 * Thin typed wrappers around `$lib/api/client`. Types alias the generated
 * `components['schemas'][...]` shapes from `schema.d.ts` so backend changes
 * flow through after a `pnpm gen:types` regen — no hand sync needed.
 */

import { api } from '$lib/api/client';
import type { components } from '$lib/api/schema';

// ── Templates ────────────────────────────────────────────────────────────

export type Template = components['schemas']['Template'];
export type RenderedTemplate = components['schemas']['RenderedTemplate'];
export type PaginatedTemplatesResponse =
	components['schemas']['PaginatedTemplatesResponse'];

export interface TemplateListQuery {
	key?: string;
	channel?: string;
	locale?: string;
	active_only?: boolean;
	page?: number;
	per_page?: number;
}

export interface CreateTemplateBody {
	key: string;
	channel: string;
	locale?: string;
	subject?: string | null;
	body_source: string;
	variables?: unknown;
	is_active?: boolean;
}

export interface UpdateTemplateBody {
	subject?: string | null;
	body_source: string;
	variables?: unknown;
	is_active?: boolean;
}

export interface PreviewBody {
	context: unknown;
}

export interface TestSendBody {
	to: string;
	context: unknown;
}

export interface TestSendResponse {
	provider_id: string;
	subject: string | null;
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

export const templates = {
	list(q: TemplateListQuery = {}): Promise<PaginatedTemplatesResponse> {
		return api.get<PaginatedTemplatesResponse>(
			`/api/admin/notifications/templates${buildQs(q as Record<string, unknown>)}`
		);
	},
	get(id: string): Promise<Template> {
		return api.get<Template>(
			`/api/admin/notifications/templates/${encodeURIComponent(id)}`
		);
	},
	create(body: CreateTemplateBody): Promise<Template> {
		return api.post<Template>('/api/admin/notifications/templates', body);
	},
	update(id: string, body: UpdateTemplateBody): Promise<Template> {
		return api.put<Template>(
			`/api/admin/notifications/templates/${encodeURIComponent(id)}`,
			body
		);
	},
	preview(id: string, body: PreviewBody): Promise<RenderedTemplate> {
		return api.post<RenderedTemplate>(
			`/api/admin/notifications/templates/${encodeURIComponent(id)}/preview`,
			body
		);
	},
	testSend(id: string, body: TestSendBody): Promise<TestSendResponse> {
		return api.post<TestSendResponse>(
			`/api/admin/notifications/templates/${encodeURIComponent(id)}/test-send`,
			body
		);
	}
};

// ── Deliveries ───────────────────────────────────────────────────────────

export type DeliveryRow = components['schemas']['DeliveryRow'];
export type PaginatedDeliveriesResponse =
	components['schemas']['PaginatedDeliveriesResponse'];

export interface DeliveryListQuery {
	status?: string;
	user_id?: string;
	page?: number;
	per_page?: number;
}

export const deliveries = {
	list(q: DeliveryListQuery = {}): Promise<PaginatedDeliveriesResponse> {
		return api.get<PaginatedDeliveriesResponse>(
			`/api/admin/notifications/deliveries${buildQs(q as Record<string, unknown>)}`
		);
	}
};

// ── Suppression ──────────────────────────────────────────────────────────

export type Suppression = components['schemas']['Suppression'];
export type PaginatedSuppressionResponse =
	components['schemas']['PaginatedSuppressionResponse'];

export interface SuppressionListQuery {
	page?: number;
	per_page?: number;
}

export interface AddSuppressionBody {
	email: string;
	reason: string;
}

export const suppression = {
	list(
		q: SuppressionListQuery = {}
	): Promise<PaginatedSuppressionResponse> {
		return api.get<PaginatedSuppressionResponse>(
			`/api/admin/notifications/suppression${buildQs(q as Record<string, unknown>)}`
		);
	},
	add(body: AddSuppressionBody): Promise<Suppression> {
		return api.post<Suppression>('/api/admin/notifications/suppression', body);
	},
	remove(email: string): Promise<{ removed: boolean }> {
		return api.post<{ removed: boolean }>(
			'/api/admin/notifications/suppression/remove',
			{ email }
		);
	}
};
