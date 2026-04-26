/**
 * Typed wrappers for the new `/api/admin/members` admin surface (ADM-10).
 * The legacy `/api/admin/members` (paginated list, role update, delete)
 * remains in `client.ts`; this module covers the indexed search + manual
 * create endpoints introduced with the search-index migration.
 */
import { api } from './client';
import type { UserResponse } from './types';

export type AdminUserRole = 'member' | 'author' | 'support' | 'admin';
export type AdminUserStatus = 'active' | 'suspended' | 'banned' | 'unverified';

export interface AdminMemberSearchQuery {
	q?: string;
	role?: AdminUserRole | '';
	status?: AdminUserStatus | '';
	limit?: number;
	offset?: number;
}

export interface AdminMemberSearchResponse {
	data: UserResponse[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

export interface CreateMemberRequest {
	email: string;
	name: string;
	role: AdminUserRole;
	temp_password?: string | undefined;
	email_verified?: boolean | undefined;
}

export interface CreateMemberResponse {
	user: UserResponse;
	requires_password_setup: boolean;
}

function qs(q: AdminMemberSearchQuery): string {
	const parts: string[] = [];
	for (const [k, v] of Object.entries(q)) {
		if (v === undefined || v === null || v === '') continue;
		parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
	}
	return parts.length ? `?${parts.join('&')}` : '';
}

export const adminMembersTyped = {
	search: (q: AdminMemberSearchQuery = {}) =>
		api.get<AdminMemberSearchResponse>(`/api/admin/members/search${qs(q)}`),
	create: (body: CreateMemberRequest) =>
		api.post<CreateMemberResponse>('/api/admin/members', body)
};
