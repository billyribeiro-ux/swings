<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api, ApiError } from '$lib/api/client';
	import type {
		BillingPortalResponse,
		SubscriptionStatusResponse,
		UserResponse
	} from '$lib/api/types';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';

	const memberId = $derived(page.params.id);

	let user = $state<UserResponse | null>(null);
	let subscription = $state<SubscriptionStatusResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let billingBusy = $state(false);
	let actionBusy = $state(false);
	let message = $state('');

	onMount(load);

	async function load() {
		loading = true;
		error = '';
		try {
			const [u, s] = await Promise.all([
				api.get<UserResponse>(`/api/admin/members/${memberId}`),
				api.get<SubscriptionStatusResponse>(`/api/admin/members/${memberId}/subscription`)
			]);
			user = u;
			subscription = s;
		} catch (e) {
			user = null;
			subscription = null;
			error = e instanceof ApiError ? e.message : 'Failed to load member';
		} finally {
			loading = false;
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function planLabel(plan: string): string {
		const p = plan.toLowerCase();
		return p === 'annual' ? 'Annual' : 'Monthly';
	}

	async function openPortal() {
		message = '';
		billingBusy = true;
		try {
			const { url } = await api.post<BillingPortalResponse>(
				`/api/admin/members/${memberId}/billing-portal`,
				{}
			);
			window.open(url, '_blank', 'noopener,noreferrer');
		} catch (e) {
			message = e instanceof ApiError ? e.message : 'Could not open portal';
		} finally {
			billingBusy = false;
		}
	}

	async function cancelSub() {
		if (!confirm('Schedule cancellation at period end for this member?')) return;
		message = '';
		actionBusy = true;
		try {
			await api.post(`/api/admin/members/${memberId}/subscription/cancel`, {});
			message = 'Cancellation scheduled in Stripe.';
			await load();
		} catch (e) {
			message = e instanceof ApiError ? e.message : 'Failed';
		} finally {
			actionBusy = false;
		}
	}

	async function resumeSub() {
		message = '';
		actionBusy = true;
		try {
			await api.post(`/api/admin/members/${memberId}/subscription/resume`, {});
			message = 'Renewal resumed in Stripe.';
			await load();
		} catch (e) {
			message = e instanceof ApiError ? e.message : 'Failed';
		} finally {
			actionBusy = false;
		}
	}
</script>

<svelte:head>
	<title>{user?.name ?? 'Member'} - Admin - Precision Options Signals</title>
</svelte:head>

<div class="member-detail">
	<a href="/admin/members" class="member-detail__back">
		<ArrowLeftIcon size={18} weight="bold" />
		<span>Members</span>
	</a>

	{#if loading}
		<p class="member-detail__loading">Loading…</p>
	{:else if error}
		<p class="member-detail__error">{error}</p>
	{:else if user}
		<h1 class="member-detail__title">{user.name}</h1>
		<p class="member-detail__meta">
			<span class="member-detail__email">{user.email}</span>
			<span class="member-detail__role">{user.role}</span>
		</p>
		<p class="member-detail__joined">Joined {formatDate(user.created_at)}</p>

		<section class="member-detail__section">
			<div class="member-detail__section-head">
				<LightningIcon size={22} weight="duotone" color="var(--color-teal)" />
				<h2 class="member-detail__h2">Subscription</h2>
			</div>

			{#if subscription?.subscription}
				{@const sub = subscription.subscription}
				<div class="sub-grid">
					<div>
						<span class="sub-grid__l">Plan</span><span class="sub-grid__v"
							>{planLabel(sub.plan)}</span
						>
					</div>
					<div>
						<span class="sub-grid__l">Status</span><span class="sub-grid__v">{sub.status}</span>
					</div>
					<div>
						<span class="sub-grid__l">Period</span>
						<span class="sub-grid__v"
							>{formatDate(sub.current_period_start)} – {formatDate(sub.current_period_end)}</span
						>
					</div>
				</div>
				<div class="member-detail__actions">
					<button
						type="button"
						class="member-detail__btn member-detail__btn--primary"
						disabled={billingBusy || actionBusy}
						onclick={openPortal}
					>
						{billingBusy ? 'Opening…' : 'Open billing portal'}
					</button>
					<button
						type="button"
						class="member-detail__btn"
						disabled={actionBusy || billingBusy}
						onclick={cancelSub}
					>
						Cancel at period end
					</button>
					<button
						type="button"
						class="member-detail__btn"
						disabled={actionBusy || billingBusy}
						onclick={resumeSub}
					>
						Resume renewal
					</button>
				</div>
				{#if message}
					<p class="member-detail__msg">{message}</p>
				{/if}
			{:else}
				<p class="member-detail__none">No subscription on file.</p>
			{/if}
		</section>
	{/if}
</div>

<style>
	.member-detail {
		max-width: 40rem;
	}

	.member-detail__back {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: var(--fs-sm);
		color: var(--color-teal-light);
		text-decoration: none;
		margin-bottom: 1.25rem;
	}

	.member-detail__back:hover {
		text-decoration: underline;
	}

	.member-detail__title {
		margin: 0 0 0.5rem;
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.member-detail__meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem;
		margin: 0 0 0.35rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}

	.member-detail__role {
		text-transform: capitalize;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
	}

	.member-detail__joined {
		margin: 0 0 2rem;
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
	}

	.member-detail__section {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
	}

	.member-detail__section-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.member-detail__h2 {
		margin: 0;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.sub-grid {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
		margin-bottom: 1rem;
	}

	.sub-grid__l {
		display: block;
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 0.15rem;
	}

	.sub-grid__v {
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
		font-weight: var(--w-medium);
	}

	.member-detail__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.member-detail__btn {
		padding: 0.5rem 0.9rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		border-radius: var(--radius-md);
		border: 1px solid rgba(255, 255, 255, 0.15);
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-200);
		cursor: pointer;
	}

	.member-detail__btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.member-detail__btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		border-color: transparent;
		color: var(--color-white);
	}

	.member-detail__msg {
		margin: 0.75rem 0 0;
		font-size: var(--fs-xs);
		color: var(--color-teal-light);
	}

	.member-detail__none {
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
		margin: 0;
	}

	.member-detail__loading,
	.member-detail__error {
		color: var(--color-grey-400);
	}

	.member-detail__error {
		color: #f87171;
	}
</style>
