/**
 * Typed wrappers for the `/api/admin/subscriptions` admin surface (ADM-11).
 *
 * Mirrors the Rust DTOs in `backend/src/handlers/admin_subscriptions.rs`.
 */
import { api } from './client';

export interface SubscriptionRow {
	id: string;
	user_id: string;
	plan_id?: string | null;
	status: string;
	current_period_start?: string | null;
	current_period_end?: string | null;
	cancel_at_period_end?: boolean | null;
	cancel_at?: string | null;
	billing_cycle_anchor?: string | null;
	stripe_subscription_id?: string | null;
	stripe_customer_id?: string | null;
	[k: string]: unknown;
}

export interface SubscriptionStatusResponse {
	subscription?: SubscriptionRow | null;
	is_active: boolean;
}

export interface MembershipRow {
	id: string;
	user_id: string;
	plan_id: string;
	granted_by: string;
	status: string;
	starts_at: string;
	ends_at?: string | null;
}

export interface UserSubscriptionView {
	subscription: SubscriptionStatusResponse;
	memberships: MembershipRow[];
}

export interface CompGrantRequest {
	user_id: string;
	plan_id: string;
	duration_days?: number;
	notes?: string;
}

export interface CompGrantResponse {
	membership_id: string;
	starts_at: string;
	ends_at?: string | null;
}

export interface ExtendRequest {
	days: number;
	notes?: string;
}

export interface ExtendResponse {
	subscription_id: string;
	previous_current_period_end: string;
	new_current_period_end: string;
}

export interface CycleOverrideRequest {
	anchor: string;
	notes?: string;
}

export interface CycleOverrideResponse {
	subscription_id: string;
	previous_anchor?: string | null;
	new_anchor: string;
}

export const adminSubs = {
	byUser: (userId: string) =>
		api.get<UserSubscriptionView>(
			`/api/admin/subscriptions/by-user/${encodeURIComponent(userId)}`
		),
	comp: (body: CompGrantRequest) =>
		api.post<CompGrantResponse>('/api/admin/subscriptions/comp', body),
	extend: (id: string, body: ExtendRequest) =>
		api.post<ExtendResponse>(
			`/api/admin/subscriptions/${encodeURIComponent(id)}/extend`,
			body
		),
	overrideCycle: (id: string, body: CycleOverrideRequest) =>
		api.post<CycleOverrideResponse>(
			`/api/admin/subscriptions/${encodeURIComponent(id)}/billing-cycle`,
			body
		)
};
