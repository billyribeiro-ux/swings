/**
 * EC-01 typed client for the product catalogue HTTP surface.
 *
 * Uses the auto-generated `schema.d.ts` (from the backend OpenAPI snapshot)
 * as the single source of truth for request/response shapes — follow the
 * FDN-02 migration pattern demonstrated in `client.ts`.
 */
import { api } from '$lib/api/client';
import type { components } from '$lib/api/schema';

export type Product = components['schemas']['Product'];
export type ProductVariant = components['schemas']['ProductVariant'];
export type DownloadableAsset = components['schemas']['DownloadableAsset'];
export type BundleItem = components['schemas']['BundleItem'];
export type BundleItemInput = components['schemas']['BundleItemInput'];
export type ProductType = components['schemas']['ProductType'];
export type ProductStatus = components['schemas']['ProductStatus'];
export type ProductDetail = components['schemas']['ProductDetail'];
export type CreateProductRequest = components['schemas']['CreateProductRequest'];
export type UpdateProductRequest = components['schemas']['UpdateProductRequest'];
export type SetStatusRequest = components['schemas']['SetStatusRequest'];
export type CreateVariantRequest = components['schemas']['CreateVariantRequest'];
export type UpdateVariantRequest = components['schemas']['UpdateVariantRequest'];
export type CreateAssetRequest = components['schemas']['CreateAssetRequest'];
export type SetBundleItemsRequest = components['schemas']['SetBundleItemsRequest'];

/** Pagination envelope shared across the admin surface. */
export interface ProductListResponse {
	data: Product[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

export interface ListParams {
	page?: number;
	per_page?: number;
	status?: ProductStatus;
	product_type?: ProductType;
	search?: string;
}

/** Serialize the list-param object into a URL querystring. */
function toQuery(params?: ListParams): string {
	if (!params) return '';
	const qs = new URLSearchParams();
	if (params.page !== undefined) qs.set('page', String(params.page));
	if (params.per_page !== undefined) qs.set('per_page', String(params.per_page));
	if (params.status) qs.set('status', params.status);
	if (params.product_type) qs.set('product_type', params.product_type);
	if (params.search) qs.set('search', params.search);
	const s = qs.toString();
	return s ? `?${s}` : '';
}

export const productsApi = {
	/** Admin list — includes draft/archived rows. */
	adminList(params?: ListParams): Promise<ProductListResponse> {
		return api.get<ProductListResponse>(`/api/admin/products${toQuery(params)}`);
	},
	/** Admin single — returns `ProductDetail` with variants + assets + bundle items. */
	adminGet(id: string): Promise<ProductDetail> {
		return api.get<ProductDetail>(`/api/admin/products/${id}`);
	},
	adminCreate(body: CreateProductRequest): Promise<Product> {
		return api.post<Product>('/api/admin/products', body);
	},
	adminUpdate(id: string, body: UpdateProductRequest): Promise<Product> {
		return api.put<Product>(`/api/admin/products/${id}`, body);
	},
	adminSetStatus(id: string, status: ProductStatus): Promise<Product> {
		return api.post<Product>(`/api/admin/products/${id}/status`, { status });
	},
	adminDelete(id: string): Promise<{ deleted: boolean }> {
		return api.del<{ deleted: boolean }>(`/api/admin/products/${id}`);
	},
	adminAddVariant(id: string, body: CreateVariantRequest): Promise<ProductVariant> {
		return api.post<ProductVariant>(`/api/admin/products/${id}/variants`, body);
	},
	adminUpdateVariant(
		id: string,
		variantId: string,
		body: UpdateVariantRequest
	): Promise<ProductVariant> {
		return api.put<ProductVariant>(`/api/admin/products/${id}/variants/${variantId}`, body);
	},
	adminDeleteVariant(id: string, variantId: string): Promise<{ deleted: boolean }> {
		return api.del<{ deleted: boolean }>(`/api/admin/products/${id}/variants/${variantId}`);
	},
	adminAddAsset(id: string, body: CreateAssetRequest): Promise<DownloadableAsset> {
		return api.post<DownloadableAsset>(`/api/admin/products/${id}/assets`, body);
	},
	adminDeleteAsset(id: string, assetId: string): Promise<{ deleted: boolean }> {
		return api.del<{ deleted: boolean }>(`/api/admin/products/${id}/assets/${assetId}`);
	},
	adminSetBundleItems(id: string, items: BundleItemInput[]): Promise<BundleItem[]> {
		return api.put<BundleItem[]>(`/api/admin/products/${id}/bundle-items`, { items });
	},
	/** Public list — published-only; Cache-Control: public, max-age=60 on the server. */
	publicList(params?: ListParams): Promise<ProductListResponse> {
		return api.get<ProductListResponse>(`/api/products${toQuery(params)}`, { skipAuth: true });
	},
	publicGet(slug: string): Promise<ProductDetail> {
		return api.get<ProductDetail>(`/api/products/${slug}`, { skipAuth: true });
	}
};
