/**
 * CONSENT-07 — admin client for the `/api/admin/consent/*` endpoints.
 *
 * Thin typed wrappers around `$lib/api/client`. The response shapes mirror
 * the Rust DTOs defined in `backend/src/handlers/admin_consent.rs`; they are
 * hand-typed here rather than generated because several of them (notably the
 * CONSENT-03 log DTO and the integrity anchor DTO) are not yet part of the
 * OpenAPI snapshot — the snapshot is regenerated in the same commit that
 * lands these DTOs, so on first run the generated types are a step behind.
 */

import { api } from '$lib/api/client';

export interface AdminBannerConfig {
	readonly id: string;
	readonly region: string;
	readonly locale: string;
	readonly version: number;
	readonly layout: string;
	readonly position: string;
	readonly theme_json: Record<string, unknown>;
	readonly copy_json: Record<string, unknown>;
	readonly is_active: boolean;
	readonly updated_at: string;
}

export interface BannerUpsertBody {
	region: string;
	locale: string;
	layout: string;
	position: string;
	copy_json: Record<string, unknown>;
	theme_json?: Record<string, unknown>;
	is_active?: boolean;
}

export interface AdminCategory {
	readonly key: string;
	readonly label: string;
	readonly description: string;
	readonly is_required: boolean;
	readonly sort_order: number;
}

export interface CategoryUpsertBody {
	key: string;
	label: string;
	description: string;
	is_required?: boolean;
	sort_order?: number;
}

export interface AdminService {
	readonly id: string;
	readonly slug: string;
	readonly name: string;
	readonly vendor: string;
	readonly category: string;
	readonly domains: readonly string[];
	readonly cookies: unknown;
	readonly privacy_url: string | null;
	readonly description: string | null;
	readonly is_active: boolean;
}

export interface ServiceUpsertBody {
	slug: string;
	name: string;
	vendor: string;
	category: string;
	domains?: string[];
	cookies?: unknown;
	privacy_url?: string | null;
	description?: string | null;
	is_active?: boolean;
}

export interface AdminPolicy {
	readonly id: string;
	readonly version: number;
	readonly markdown: string;
	readonly locale: string;
	readonly effective_at: string;
	readonly created_at: string;
}

export interface PolicyCreateBody {
	markdown: string;
	locale?: string;
}

export interface ConsentLogRow {
	readonly id: string;
	readonly subject_id: string | null;
	readonly action: string;
	readonly categories: Record<string, boolean>;
	readonly created_at: string;
}

export interface ConsentLogResponse {
	readonly rows: readonly ConsentLogRow[];
	readonly table_present: boolean;
	readonly total: number;
}

export interface IntegrityAnchor {
	readonly id: string;
	readonly anchor_hash: string;
	readonly record_count: number;
	readonly window_start_at: string | null;
	readonly window_end_at: string | null;
	readonly anchored_at: string;
}

// ── Banners ──────────────────────────────────────────────────────────────

export function listBanners(): Promise<AdminBannerConfig[]> {
	return api.get<AdminBannerConfig[]>('/api/admin/consent/banners');
}

export function getBanner(id: string): Promise<AdminBannerConfig> {
	return api.get<AdminBannerConfig>(`/api/admin/consent/banners/${encodeURIComponent(id)}`);
}

export function createBanner(body: BannerUpsertBody): Promise<AdminBannerConfig> {
	return api.post<AdminBannerConfig>('/api/admin/consent/banners', body);
}

export function updateBanner(id: string, body: BannerUpsertBody): Promise<AdminBannerConfig> {
	return api.put<AdminBannerConfig>(`/api/admin/consent/banners/${encodeURIComponent(id)}`, body);
}

// ── Categories ───────────────────────────────────────────────────────────

export function listCategories(): Promise<AdminCategory[]> {
	return api.get<AdminCategory[]>('/api/admin/consent/categories');
}

export function createCategory(body: CategoryUpsertBody): Promise<AdminCategory> {
	return api.post<AdminCategory>('/api/admin/consent/categories', body);
}

export function updateCategory(key: string, body: CategoryUpsertBody): Promise<AdminCategory> {
	return api.put<AdminCategory>(`/api/admin/consent/categories/${encodeURIComponent(key)}`, body);
}

// ── Services ─────────────────────────────────────────────────────────────

export function listServices(): Promise<AdminService[]> {
	return api.get<AdminService[]>('/api/admin/consent/services');
}

export function createService(body: ServiceUpsertBody): Promise<AdminService> {
	return api.post<AdminService>('/api/admin/consent/services', body);
}

export function updateService(id: string, body: ServiceUpsertBody): Promise<AdminService> {
	return api.put<AdminService>(`/api/admin/consent/services/${encodeURIComponent(id)}`, body);
}

// ── Policies ─────────────────────────────────────────────────────────────

export function listPolicies(): Promise<AdminPolicy[]> {
	return api.get<AdminPolicy[]>('/api/admin/consent/policies');
}

export function createPolicy(body: PolicyCreateBody): Promise<AdminPolicy> {
	return api.post<AdminPolicy>('/api/admin/consent/policies', body);
}

// ── Log (CONSENT-03 read-only view) ──────────────────────────────────────

export function listLog(limit?: number, offset?: number): Promise<ConsentLogResponse> {
	const params = new URLSearchParams();
	if (limit != null) params.set('limit', String(limit));
	if (offset != null) params.set('offset', String(offset));
	const qs = params.toString() ? `?${params.toString()}` : '';
	return api.get<ConsentLogResponse>(`/api/admin/consent/log${qs}`);
}

// ── Integrity anchors ────────────────────────────────────────────────────

export function listIntegrity(): Promise<IntegrityAnchor[]> {
	return api.get<IntegrityAnchor[]>('/api/admin/consent/integrity');
}
