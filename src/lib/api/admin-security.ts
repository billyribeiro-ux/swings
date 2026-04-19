/**
 * Typed wrappers for the `/api/admin/security/*` admin surfaces:
 *
 *   - IP allowlist        (ADM-06)  → `/api/admin/security/ip-allowlist`
 *   - Impersonation       (ADM-07)  → `/api/admin/security/impersonation`
 *   - Role/permission     (ADM-09)  → `/api/admin/security/roles`
 *
 * Shapes mirror the Rust DTOs defined in `backend/src/handlers/admin_*.rs`
 * and `backend/src/security/*.rs`. Each function returns the parsed JSON
 * body and lets {@link ApiError} propagate so the calling component can
 * decide how to surface the Problem document.
 */
import { api } from './client';

// ── IP allowlist ────────────────────────────────────────────────────────

export interface AllowlistEntry {
	id: string;
	cidr: string;
	label: string;
	is_active: boolean;
	created_by: string;
	created_at: string;
	updated_at: string;
}

export interface AllowlistResponse {
	data: AllowlistEntry[];
	total: number;
}

export interface CreateAllowlistInput {
	cidr: string;
	label: string;
	is_active?: boolean;
}

export const ipAllowlist = {
	list: () => api.get<AllowlistResponse>('/api/admin/security/ip-allowlist'),
	create: (input: CreateAllowlistInput) =>
		api.post<AllowlistEntry>('/api/admin/security/ip-allowlist', input),
	remove: (id: string) =>
		api.del<{ ok: boolean }>(`/api/admin/security/ip-allowlist/${encodeURIComponent(id)}`),
	toggle: (id: string, isActive: boolean) =>
		api.post<AllowlistEntry>(
			`/api/admin/security/ip-allowlist/${encodeURIComponent(id)}/toggle`,
			{ is_active: isActive }
		)
};

// ── Impersonation ───────────────────────────────────────────────────────

export interface ImpersonationSession {
	id: string;
	actor_user_id: string;
	actor_role: string;
	target_user_id: string;
	reason: string;
	issued_at: string;
	expires_at: string;
	revoked_at?: string | null;
	revoked_by?: string | null;
	revoke_reason?: string | null;
	ip_address?: string | null;
	user_agent?: string | null;
}

export interface ImpersonationListResponse {
	data: ImpersonationSession[];
	total: number;
	next_cursor?: string | null;
}

export interface MintImpersonationInput {
	target_user_id: string;
	reason: string;
	ttl_minutes?: number;
}

export interface MintImpersonationResponse {
	access_token: string;
	expires_at: string;
	session: ImpersonationSession;
}

export const impersonation = {
	list: (after?: string) =>
		api.get<ImpersonationListResponse>(
			after
				? `/api/admin/security/impersonation?after=${encodeURIComponent(after)}`
				: '/api/admin/security/impersonation'
		),
	get: (id: string) =>
		api.get<ImpersonationSession>(
			`/api/admin/security/impersonation/${encodeURIComponent(id)}`
		),
	mint: (input: MintImpersonationInput) =>
		api.post<MintImpersonationResponse>('/api/admin/security/impersonation', input),
	revoke: (id: string, reason: string) =>
		api.post<ImpersonationSession>(
			`/api/admin/security/impersonation/${encodeURIComponent(id)}/revoke`,
			{ reason }
		)
};

// ── Role / permission matrix ────────────────────────────────────────────

export interface PermissionRow {
	key: string;
	description: string;
}

export interface PermissionsResponse {
	data: PermissionRow[];
	total: number;
}

export interface RolePermPair {
	role: string;
	permission: string;
}

export interface MatrixResponse {
	matrix: RolePermPair[];
	roles: string[];
	permissions: PermissionRow[];
}

export interface ReplaceRoleRequest {
	permissions: string[];
}

export const roleMatrix = {
	list: () => api.get<MatrixResponse>('/api/admin/security/roles'),
	listPermissions: () =>
		api.get<PermissionsResponse>('/api/admin/security/roles/permissions'),
	grant: (role: string, permission: string) =>
		api.post<RolePermPair>(
			`/api/admin/security/roles/${encodeURIComponent(role)}/${encodeURIComponent(permission)}`
		),
	revoke: (role: string, permission: string) =>
		api.del<{ role: string; permission: string; removed: boolean }>(
			`/api/admin/security/roles/${encodeURIComponent(role)}/${encodeURIComponent(permission)}`
		),
	replace: (role: string, permissions: string[]) =>
		api.put<{ role: string; permissions: string[]; count: number }>(
			`/api/admin/security/roles/${encodeURIComponent(role)}`,
			{ permissions }
		),
	reload: () =>
		api.post<{ reloaded: number }>('/api/admin/security/roles/_reload')
};

// ── Audit log (ADM-14) ──────────────────────────────────────────────────

export interface AuditRow {
	id: string;
	actor_id: string;
	actor_role: string;
	action: string;
	target_kind: string;
	target_id?: string | null;
	ip_address?: string | null;
	user_agent?: string | null;
	metadata: unknown;
	created_at: string;
}

export interface AuditListEnvelope {
	data: AuditRow[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

export interface AuditListQuery {
	q?: string;
	actor_id?: string;
	action?: string;
	target_kind?: string;
	target_id?: string;
	metadata_contains?: string;
	from?: string;
	to?: string;
	limit?: number;
	offset?: number;
}

function audit_qs(q: AuditListQuery): string {
	const parts: string[] = [];
	for (const [k, v] of Object.entries(q)) {
		if (v === undefined || v === null || v === '') continue;
		parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
	}
	return parts.length ? `?${parts.join('&')}` : '';
}

export const auditLog = {
	list: (q: AuditListQuery = {}) =>
		api.get<AuditListEnvelope>(`/api/admin/audit${audit_qs(q)}`),
	get: (id: string) => api.get<AuditRow>(`/api/admin/audit/${encodeURIComponent(id)}`),
	exportCsvUrl: (q: AuditListQuery = {}) => `/api/admin/audit/export.csv${audit_qs(q)}`
};

// ── DSAR (ADM-13) ───────────────────────────────────────────────────────

/**
 * A DSAR job row, as returned by the admin DSAR endpoints.
 *
 * Asynchronous exports (ADM-17 + ADM-19) introduce three optional
 * fields that the synchronous path never populates:
 *
 *   - `artifact_kind` — `inline`, `r2`, or `local`. Drives the
 *     download UX: `inline` is a `data:` URI on `artifact_url`,
 *     `r2` is a presigned URL on `artifact_url`, and `local`
 *     requires hitting the `/jobs/{id}/artifact` streamer with
 *     bearer auth.
 *   - `artifact_storage_key` — opaque storage key (R2 object
 *     key or relative local path). Operator surfaces show this for
 *     diagnostic reasons; downloads should use `artifact_url` or
 *     the streamer endpoint, never this key directly.
 *   - `artifact_expires_at` — TTL for the artefact. After this
 *     timestamp the TTL sweep (ADM-19) deletes the underlying
 *     object and NULLs all three columns. The UI uses it to render
 *     a relative "expires in 23h" hint and to grey-out download
 *     buttons on stale rows.
 */
export interface DsarJob {
	id: string;
	target_user_id: string;
	kind: 'export' | 'erase' | string;
	status: 'pending' | 'composing' | 'completed' | 'cancelled' | 'failed' | string;
	requested_by: string;
	request_reason: string;
	approved_by?: string | null;
	approval_reason?: string | null;
	approved_at?: string | null;
	artifact_url?: string | null;
	artifact_kind?: 'inline' | 'r2' | 'local' | null;
	artifact_storage_key?: string | null;
	artifact_expires_at?: string | null;
	erasure_summary?: unknown | null;
	completed_at?: string | null;
	failure_reason?: string | null;
	created_at: string;
	updated_at: string;
}

export interface DsarJobListEnvelope {
	data: DsarJob[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

export interface DsarListQuery {
	status?: string;
	kind?: string;
	target_user_id?: string;
	limit?: number;
	offset?: number;
}

export interface DsarExportRequest {
	target_user_id: string;
	reason: string;
	/**
	 * When `true`, the backend queues a `pending` job and returns
	 * `202 Accepted` with `export: null`. The dsar-worker then
	 * composes the export, persists it to R2 / local disk, and
	 * stamps `artifact_*` on the row.
	 *
	 * When omitted / `false`, the legacy synchronous path is used:
	 * the export is composed inline and returned as a `data:` URI
	 * on `job.artifact_url` plus a parsed JSON snapshot in `export`.
	 */
	async?: boolean;
}

export interface DsarExportResponse {
	job: DsarJob;
	/**
	 * Inline export snapshot. `null` when the request was async —
	 * the worker writes the artefact to storage and the operator
	 * downloads it later via the table row's "Download" action.
	 */
	export: unknown | null;
}

export interface DsarEraseRequestBody {
	target_user_id: string;
	reason: string;
}

export interface DsarEraseApproveBody {
	approval_reason: string;
}

export interface DsarTombstoneSummary {
	target_user_id: string;
	tokens_deleted: number;
	password_resets_deleted: number;
	preferences_deleted: number;
	[k: string]: unknown;
}

export interface DsarEraseApproveResponse {
	job: DsarJob;
	summary: DsarTombstoneSummary;
}

function dsar_qs(q: DsarListQuery): string {
	const parts: string[] = [];
	for (const [k, v] of Object.entries(q)) {
		if (v === undefined || v === null || v === '') continue;
		parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
	}
	return parts.length ? `?${parts.join('&')}` : '';
}

// ── App settings (ADM-08) ──────────────────────────────────────────────

export type SettingType = 'string' | 'int' | 'bool' | 'json' | 'secret';

export interface SettingView {
	key: string;
	value: unknown;
	value_type: SettingType;
	is_secret: boolean;
	description?: string | null;
	category: string;
	updated_at: string;
	updated_by?: string | null;
}

export interface SettingListResponse {
	data: SettingView[];
	total: number;
}

export interface SettingGetResponse extends SettingView {
	revealed_value?: unknown;
}

export interface SettingUpsertRequest {
	value_type?: SettingType;
	is_secret?: boolean;
	description?: string;
	category?: string;
	value: unknown;
}

export const appSettings = {
	list: () => api.get<SettingListResponse>('/api/admin/settings'),
	get: (key: string, reveal = false) =>
		api.get<SettingGetResponse>(
			`/api/admin/settings/${encodeURIComponent(key)}${reveal ? '?reveal=true' : ''}`
		),
	upsert: (key: string, body: SettingUpsertRequest) =>
		api.put<SettingView>(`/api/admin/settings/${encodeURIComponent(key)}`, body),
	reload: () => api.post<{ reloaded: number }>('/api/admin/settings/_reload')
};

export const dsarAdmin = {
	listJobs: (q: DsarListQuery = {}) =>
		api.get<DsarJobListEnvelope>(`/api/admin/dsar/jobs${dsar_qs(q)}`),
	getJob: (id: string) => api.get<DsarJob>(`/api/admin/dsar/jobs/${encodeURIComponent(id)}`),
	createExport: (body: DsarExportRequest) =>
		api.post<DsarExportResponse>('/api/admin/dsar/jobs/export', body),
	requestErase: (body: DsarEraseRequestBody) =>
		api.post<DsarJob>('/api/admin/dsar/jobs/erase/request', body),
	approveErase: (id: string, body: DsarEraseApproveBody) =>
		api.post<DsarEraseApproveResponse>(
			`/api/admin/dsar/jobs/${encodeURIComponent(id)}/erase/approve`,
			body
		),
	cancelJob: (id: string, reason?: string) =>
		api.post<DsarJob>(`/api/admin/dsar/jobs/${encodeURIComponent(id)}/cancel`, {
			reason
		}),
	/**
	 * Stream an async-export artefact whose storage backend is
	 * `local`. The `r2` path advertises a presigned URL on
	 * `artifact_url` directly; the `inline` path carries a `data:`
	 * URI on `artifact_url`. This streamer is only required when
	 * the deployment's `MediaBackend` is the local filesystem.
	 *
	 * Returns the response body as a `Blob` plus the suggested
	 * filename parsed from `Content-Disposition` (the backend sets
	 * `attachment; filename="dsar-{id}.json"`).
	 */
	streamArtifact: (id: string) =>
		api.getBlob(`/api/admin/dsar/jobs/${encodeURIComponent(id)}/artifact`)
};
