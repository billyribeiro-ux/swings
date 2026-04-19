/**
 * Typed wrappers for the `/api/admin/orders` admin surface (ADM-12).
 *
 * Mirrors the Rust DTOs in `backend/src/handlers/admin_orders.rs` and the
 * `Order`/`OrderItem`/`OrderRefund`/`OrderNote` rows from
 * `backend/src/commerce/orders.rs`.
 */
import { api } from './client';

export interface Order {
	id: string;
	number: string;
	user_id?: string | null;
	cart_id?: string | null;
	status: string;
	currency: string;
	subtotal_cents: number;
	discount_cents: number;
	tax_cents: number;
	total_cents: number;
	email: string;
	stripe_payment_intent_id?: string | null;
	stripe_customer_id?: string | null;
	idempotency_key?: string | null;
	metadata: unknown;
	placed_at?: string | null;
	completed_at?: string | null;
	created_at: string;
	updated_at: string;
}

export interface OrderItem {
	id: string;
	order_id: string;
	product_id: string;
	variant_id?: string | null;
	sku?: string | null;
	name: string;
	quantity: number;
	unit_price_cents: number;
	line_total_cents: number;
	metadata: unknown;
	created_at: string;
}

export interface OrderRefund {
	id: string;
	order_id: string;
	amount_cents: number;
	reason?: string | null;
	stripe_refund_id?: string | null;
	created_by?: string | null;
	created_at: string;
}

export interface OrderNote {
	id: string;
	order_id: string;
	author_id?: string | null;
	kind: string;
	body: string;
	created_at: string;
}

export interface OrderListEnvelope {
	data: Order[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

export interface OrderDetail {
	order: Order;
	items: OrderItem[];
	refunds: OrderRefund[];
	notes: OrderNote[];
	refunded_cents: number;
	remaining_refundable_cents: number;
}

export interface OrderListQuery {
	q?: string;
	status?: string;
	limit?: number;
	offset?: number;
}

export interface ManualOrderItemInput {
	product_id: string;
	quantity: number;
	unit_price_cents: number;
	name: string;
	sku?: string;
}

export interface ManualOrderRequest {
	email: string;
	user_id?: string;
	currency?: string;
	items: ManualOrderItemInput[];
	discount_cents?: number;
	tax_cents?: number;
	mark_completed?: boolean;
	notes?: string;
}

export interface VoidRequest {
	reason?: string;
}

export interface RefundRequest {
	amount_cents: number;
	reason?: string;
	stripe_refund_id?: string;
}

export interface RefundResponse {
	refund: OrderRefund;
	order_marked_refunded: boolean;
	remaining_refundable_cents: number;
}

function qs(q: OrderListQuery): string {
	const parts: string[] = [];
	for (const [k, v] of Object.entries(q)) {
		if (v === undefined || v === null || v === '') continue;
		parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
	}
	return parts.length ? `?${parts.join('&')}` : '';
}

export const adminOrders = {
	list: (q: OrderListQuery = {}) =>
		api.get<OrderListEnvelope>(`/api/admin/orders${qs(q)}`),
	get: (id: string) => api.get<OrderDetail>(`/api/admin/orders/${encodeURIComponent(id)}`),
	createManual: (body: ManualOrderRequest) => api.post<OrderDetail>('/api/admin/orders', body),
	void: (id: string, body: VoidRequest) =>
		api.post<OrderDetail>(`/api/admin/orders/${encodeURIComponent(id)}/void`, body),
	refund: (id: string, body: RefundRequest) =>
		api.post<RefundResponse>(`/api/admin/orders/${encodeURIComponent(id)}/refund`, body),
	exportCsvUrl: (q: OrderListQuery = {}) => `/api/admin/orders/export.csv${qs(q)}`
};

export function formatMoney(cents: number, currency = 'usd'): string {
	const value = cents / 100;
	try {
		return new Intl.NumberFormat(undefined, {
			style: 'currency',
			currency: currency.toUpperCase()
		}).format(value);
	} catch {
		return `${currency.toUpperCase()} ${value.toFixed(2)}`;
	}
}
