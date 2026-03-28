<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { SubscriptionResponse } from '$lib/api/types';
	import UserCircle from 'phosphor-svelte/lib/UserCircle';
	import Lightning from 'phosphor-svelte/lib/Lightning';

	let name = $state(auth.user?.name ?? '');
	let saving = $state(false);
	let saved = $state(false);
	let error = $state('');
	let subscription = $state<SubscriptionResponse | null>(null);

	onMount(async () => {
		try {
			subscription = await api.get<SubscriptionResponse>('/api/member/subscription');
		} catch {
			// silently handle
		}
	});

	async function handleSave(e: Event) {
		e.preventDefault();
		saving = true;
		saved = false;
		error = '';

		try {
			const res = await api.put<{
				id: string;
				email: string;
				name: string;
				role: string;
				avatar_url: string | null;
				created_at: string;
			}>('/api/member/profile', { name });
			auth.setUser({
				...auth.user!,
				name: res.name
			});
			saved = true;
			setTimeout(() => (saved = false), 3000);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to update profile';
		} finally {
			saving = false;
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function planLabel(plan: string): string {
		return plan === 'annual' ? 'Annual ($399/yr)' : 'Monthly ($49/mo)';
	}

	function statusColor(status: string): string {
		if (status === 'active' || status === 'trialing') return '#22c55e';
		if (status === 'past_due') return '#f59e0b';
		return '#ef4444';
	}
</script>

<svelte:head>
	<title>Account - Explosive Swings</title>
</svelte:head>

<div class="account">
	<h2 class="account__title">Account Settings</h2>

	<!-- Profile Section -->
	<section class="account__section">
		<div class="account__section-header">
			<UserCircle size={22} weight="duotone" color="var(--color-teal)" />
			<h3 class="account__section-title">Profile</h3>
		</div>

		<form onsubmit={handleSave} class="account__form">
			<div class="account__field">
				<label for="name" class="account__label">Name</label>
				<input id="name" type="text" bind:value={name} class="account__input" />
			</div>

			<div class="account__field">
				<span class="account__label">Email</span>
				<p class="account__static">{auth.user?.email}</p>
			</div>

			<div class="account__field">
				<span class="account__label">Member Since</span>
				<p class="account__static">
					{auth.user?.created_at ? formatDate(auth.user.created_at) : '-'}
				</p>
			</div>

			{#if error}
				<p class="account__error">{error}</p>
			{/if}

			<button type="submit" disabled={saving} class="account__save">
				{#if saving}
					Saving...
				{:else if saved}
					Saved!
				{:else}
					Save Changes
				{/if}
			</button>
		</form>
	</section>

	<!-- Subscription Section -->
	<section class="account__section">
		<div class="account__section-header">
			<Lightning size={22} weight="duotone" color="var(--color-teal)" />
			<h3 class="account__section-title">Subscription</h3>
		</div>

		{#if subscription?.subscription}
			{@const sub = subscription.subscription}
			<div class="sub-card">
				<div class="sub-card__row">
					<span class="sub-card__label">Plan</span>
					<span class="sub-card__value">{planLabel(sub.plan)}</span>
				</div>
				<div class="sub-card__row">
					<span class="sub-card__label">Status</span>
					<span class="sub-card__status" style="color: {statusColor(sub.status)}">
						{sub.status}
					</span>
				</div>
				<div class="sub-card__row">
					<span class="sub-card__label">Current Period</span>
					<span class="sub-card__value">
						{formatDate(sub.current_period_start)} - {formatDate(sub.current_period_end)}
					</span>
				</div>
			</div>
		{:else}
			<div class="sub-card sub-card--empty">
				<p>No active subscription.</p>
				<a href="/#pricing" class="sub-card__cta">View Plans</a>
			</div>
		{/if}
	</section>
</div>

<style>
	.account__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 2rem;
	}

	.account__section {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.account__section-header {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		margin-bottom: 1.25rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.account__section-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.account__form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.account__field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.account__label {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.account__input {
		padding: 0.65rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
		transition: border-color 200ms var(--ease-out);
	}

	.account__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.account__static {
		color: var(--color-grey-300);
		font-size: var(--fs-base);
	}

	.account__error {
		color: #fca5a5;
		font-size: var(--fs-sm);
	}

	.account__save {
		align-self: flex-start;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}

	.account__save:hover:not(:disabled) {
		opacity: 0.9;
	}

	.account__save:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.sub-card {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.sub-card--empty {
		align-items: center;
		text-align: center;
		color: var(--color-grey-400);
		gap: 1rem;
	}

	.sub-card__cta {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		text-decoration: none;
	}

	.sub-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.sub-card__label {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
	}

	.sub-card__value {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.sub-card__status {
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
		text-transform: capitalize;
	}
</style>
