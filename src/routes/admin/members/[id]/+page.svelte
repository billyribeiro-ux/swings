<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { api, ApiError } from '$lib/api/client';
	import type {
		BillingPortalResponse,
		MemberDetailResponse,
		UpdateMemberRequest,
		UserResponse
	} from '$lib/api/types';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import EnvelopeIcon from 'phosphor-svelte/lib/EnvelopeIcon';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import ProhibitIcon from 'phosphor-svelte/lib/ProhibitIcon';
	import ClockCountdownIcon from 'phosphor-svelte/lib/ClockCountdownIcon';
	import KeyIcon from 'phosphor-svelte/lib/KeyIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import MapPinIcon from 'phosphor-svelte/lib/MapPinIcon';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';

	const memberId = $derived(page.params.id);

	let detail = $state<MemberDetailResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let billingBusy = $state(false);

	// Edit-mode shadow state for the two editable sections. We mirror
	// the loaded user into local fields so the user can edit without
	// accidentally clobbering the detail object before they hit save.
	let profileDraft = $state({ name: '', email: '', phone: '' });
	let addressDraft = $state({
		line1: '',
		line2: '',
		city: '',
		state: '',
		postal_code: '',
		country: ''
	});
	let savingProfile = $state(false);
	let savingAddress = $state(false);

	const profileDirty = $derived(
		!!detail?.user &&
			(profileDraft.name !== (detail.user.name ?? '') ||
				profileDraft.email !== (detail.user.email ?? '') ||
				profileDraft.phone !== (detail.user.phone ?? ''))
	);

	const addressDirty = $derived(
		!!detail?.user &&
			(addressDraft.line1 !== (detail.user.billing_line1 ?? '') ||
				addressDraft.line2 !== (detail.user.billing_line2 ?? '') ||
				addressDraft.city !== (detail.user.billing_city ?? '') ||
				addressDraft.state !== (detail.user.billing_state ?? '') ||
				addressDraft.postal_code !== (detail.user.billing_postal_code ?? '') ||
				addressDraft.country !== (detail.user.billing_country ?? ''))
	);

	type Lifecycle = 'banned' | 'suspended' | 'active';
	const lifecycle = $derived<Lifecycle>(
		detail?.user.banned_at ? 'banned' : detail?.user.suspended_at ? 'suspended' : 'active'
	);

	onMount(load);

	function syncDrafts(user: UserResponse) {
		profileDraft = {
			name: user.name ?? '',
			email: user.email ?? '',
			phone: user.phone ?? ''
		};
		addressDraft = {
			line1: user.billing_line1 ?? '',
			line2: user.billing_line2 ?? '',
			city: user.billing_city ?? '',
			state: user.billing_state ?? '',
			postal_code: user.billing_postal_code ?? '',
			country: user.billing_country ?? ''
		};
	}

	async function load() {
		loading = true;
		error = '';
		try {
			const d = await api.get<MemberDetailResponse>(`/api/admin/members/${memberId}/detail`);
			detail = d;
			syncDrafts(d.user);
		} catch (e) {
			detail = null;
			error = e instanceof ApiError ? e.message : 'Failed to load member';
		} finally {
			loading = false;
		}
	}

	function formatDate(dateStr: string | null | undefined): string {
		if (!dateStr) return '—';
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function formatDateTime(dateStr: string | null | undefined): string {
		if (!dateStr) return '—';
		return new Date(dateStr).toLocaleString('en-US');
	}

	function formatCents(amount: number | null, currency: string | null): string {
		if (amount == null) return '—';
		const cur = (currency ?? 'USD').toUpperCase();
		return new Intl.NumberFormat('en-US', { style: 'currency', currency: cur }).format(
			amount / 100
		);
	}

	function planLabel(plan: string): string {
		return plan.toLowerCase() === 'annual' ? 'Annual' : 'Monthly';
	}

	function statusLabel(state: Lifecycle): string {
		return state === 'banned' ? 'Banned' : state === 'suspended' ? 'Suspended' : 'Active';
	}

	async function saveProfile() {
		if (!detail || !profileDirty || savingProfile) return;
		savingProfile = true;
		try {
			const body: UpdateMemberRequest = {
				name: profileDraft.name.trim(),
				email: profileDraft.email.trim(),
				phone: profileDraft.phone.trim()
			};
			const updated = await api.patch<UserResponse>(`/api/admin/members/${memberId}`, body);
			detail = { ...detail, user: updated };
			syncDrafts(updated);
			toast.success('Profile updated');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to save profile');
		} finally {
			savingProfile = false;
		}
	}

	async function saveAddress() {
		if (!detail || !addressDirty || savingAddress) return;
		savingAddress = true;
		try {
			const body: UpdateMemberRequest = {
				billing_address: {
					line1: addressDraft.line1.trim() || undefined,
					line2: addressDraft.line2.trim() || undefined,
					city: addressDraft.city.trim() || undefined,
					state: addressDraft.state.trim() || undefined,
					postal_code: addressDraft.postal_code.trim() || undefined,
					country: addressDraft.country.trim() || undefined
				}
			};
			const updated = await api.patch<UserResponse>(`/api/admin/members/${memberId}`, body);
			detail = { ...detail, user: updated };
			syncDrafts(updated);
			toast.success('Billing address updated — Stripe sync queued');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to save address');
		} finally {
			savingAddress = false;
		}
	}

	async function openPortal() {
		if (!detail) return;
		billingBusy = true;
		try {
			const { url } = await api.post<BillingPortalResponse>(
				`/api/admin/members/${memberId}/billing-portal`,
				{}
			);
			window.open(url, '_blank', 'noopener,noreferrer');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Could not open portal');
		} finally {
			billingBusy = false;
		}
	}

	async function banOrUnban() {
		if (!detail) return;
		const u = detail.user;
		if (u.banned_at) {
			const ok = await confirmDialog({
				title: 'Lift ban?',
				message: `Reinstate ${u.name}'s account access?`,
				confirmLabel: 'Lift ban',
				variant: 'warning'
			});
			if (!ok) return;
			try {
				await api.post(`/api/admin/members/${memberId}/unban`, {});
				toast.success('Ban lifted');
				await load();
			} catch (e) {
				toast.error(e instanceof ApiError ? e.message : 'Failed to lift ban');
			}
			return;
		}
		const reason = window.prompt(`Reason for banning ${u.name}? (optional)`) ?? '';
		const ok = await confirmDialog({
			title: 'Ban member?',
			message: `Banning ${u.name} revokes their session and immediately cancels any active subscription.`,
			confirmLabel: 'Ban member',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.post(`/api/admin/members/${memberId}/ban`, { reason });
			toast.success('Member banned');
			await load();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Ban failed');
		}
	}

	async function suspendOrUnsuspend() {
		if (!detail) return;
		const u = detail.user;
		if (u.suspended_at) {
			const ok = await confirmDialog({
				title: 'Lift suspension?',
				message: `Restore ${u.name}'s ability to log in?`,
				confirmLabel: 'Lift suspension',
				variant: 'warning'
			});
			if (!ok) return;
			try {
				await api.post(`/api/admin/members/${memberId}/unsuspend`, {});
				toast.success('Suspension lifted');
				await load();
			} catch (e) {
				toast.error(e instanceof ApiError ? e.message : 'Failed to lift suspension');
			}
			return;
		}
		const reason = window.prompt(`Reason for suspending ${u.name}? (optional)`) ?? '';
		const days = window.prompt('Suspension duration in days (blank = open-ended):', '');
		let until: string | undefined;
		if (days && Number.parseInt(days, 10) > 0) {
			const ts = new Date(Date.now() + Number.parseInt(days, 10) * 86400000);
			until = ts.toISOString();
		}
		try {
			await api.post(`/api/admin/members/${memberId}/suspend`, { reason, until });
			toast.success(
				until
					? `Suspended until ${new Date(until).toLocaleDateString()}`
					: 'Member suspended'
			);
			await load();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Suspension failed');
		}
	}

	async function resetPassword() {
		if (!detail) return;
		const ok = await confirmDialog({
			title: 'Send reset email?',
			message: `Email ${detail.user.email} a new password reset link and revoke their active sessions.`,
			confirmLabel: 'Send reset',
			variant: 'warning'
		});
		if (!ok) return;
		try {
			await api.post(`/api/admin/members/${memberId}/force-password-reset`, {});
			toast.success('Reset email dispatched');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to send reset');
		}
	}

	async function verifyEmail() {
		if (!detail) return;
		try {
			const updated = await api.post<UserResponse>(
				`/api/admin/members/${memberId}/verify-email`,
				{}
			);
			detail = { ...detail, user: updated };
			toast.success('Email marked verified');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to verify email');
		}
	}

	async function deleteMember() {
		if (!detail) return;
		const ok = await confirmDialog({
			title: 'Delete member?',
			message: `Permanently remove ${detail.user.name}? This cancels any active Stripe subscription and cannot be undone.`,
			confirmLabel: 'Delete',
			cancelLabel: 'Keep account',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.del(`/api/admin/members/${memberId}`);
			toast.success('Member deleted');
			await goto('/admin/members');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Delete failed');
		}
	}

	function actionLabel(action: string): string {
		// Friendly labels for the activity timeline; falls back to the raw
		// dot-delimited verb when we don't have a translation yet.
		const map: Record<string, string> = {
			'user.suspend': 'Suspended',
			'user.unsuspend': 'Suspension lifted',
			'user.reactivate': 'Reactivated',
			'user.ban': 'Banned',
			'user.unban': 'Ban lifted',
			'user.update': 'Profile updated',
			'user.role.update': 'Role changed',
			'user.delete': 'Deleted',
			'user.email.verify': 'Email verified',
			'user.force_password_reset': 'Password reset sent',
			'admin.member.create': 'Account created',
			'subscription.cancel_at_period_end': 'Subscription scheduled to cancel',
			'subscription.resume': 'Subscription renewal resumed'
		};
		return map[action] ?? action;
	}
</script>

<svelte:head>
	<title>{detail?.user.name ?? 'Member'} - Admin - Precision Options Signals</title>
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
	{:else if detail}
		{@const u = detail.user}
		<div class="member-detail__layout">
			<aside class="member-detail__sidebar">
				<div class="member-detail__id">
					<span class="member-detail__avatar" aria-hidden="true">
						{u.name?.[0]?.toUpperCase() || '?'}
					</span>
					<h1 class="member-detail__name">{u.name}</h1>
					<p class="member-detail__email">{u.email}</p>
					<div class="member-detail__pills">
						<span class={['member-detail__pill', `member-detail__pill--${lifecycle}`]}>
							{statusLabel(lifecycle)}
						</span>
						<span
							class={[
								'member-detail__role',
								u.role === 'admin'
									? 'member-detail__role--admin'
									: 'member-detail__role--member'
							]}
						>
							{u.role}
						</span>
					</div>
					{#if u.banned_at && u.ban_reason}
						<p class="member-detail__reason">Reason: {u.ban_reason}</p>
					{:else if u.suspended_at && u.suspension_reason}
						<p class="member-detail__reason">
							Reason: {u.suspension_reason}
							{#if u.suspended_until}
								<br />Until {formatDateTime(u.suspended_until)}
							{/if}
						</p>
					{/if}
				</div>

				<dl class="member-detail__meta">
					<div>
						<dt>Joined</dt>
						<dd>{formatDate(u.created_at)}</dd>
					</div>
					<div>
						<dt>Email verified</dt>
						<dd>
							{#if u.email_verified_at}
								<CheckCircleIcon size={14} weight="fill" />
								{formatDate(u.email_verified_at)}
							{:else}
								Not verified
							{/if}
						</dd>
					</div>
				</dl>

				<div class="member-detail__sidebar-actions">
					<Tooltip label="Edit lives in the Profile section to the right">
						<button class="member-detail__action" disabled>
							<UserCircleIcon size={16} weight="bold" />
							<span>Edit profile</span>
						</button>
					</Tooltip>
					<Tooltip label={u.suspended_at ? 'Lift suspension' : 'Suspend / timeout'}>
						<button
							class="member-detail__action member-detail__action--warn"
							onclick={suspendOrUnsuspend}
						>
							<ClockCountdownIcon size={16} weight="bold" />
							<span>{u.suspended_at ? 'Lift suspension' : 'Suspend'}</span>
						</button>
					</Tooltip>
					<Tooltip label={u.banned_at ? 'Lift ban' : 'Ban member'}>
						<button
							class="member-detail__action member-detail__action--danger"
							onclick={banOrUnban}
						>
							<ProhibitIcon size={16} weight="bold" />
							<span>{u.banned_at ? 'Lift ban' : 'Ban'}</span>
						</button>
					</Tooltip>
					<Tooltip label="Send password reset email">
						<button class="member-detail__action" onclick={resetPassword}>
							<KeyIcon size={16} weight="bold" />
							<span>Reset password</span>
						</button>
					</Tooltip>
					{#if !u.email_verified_at}
						<Tooltip label="Mark email as verified without OTP">
							<button class="member-detail__action" onclick={verifyEmail}>
								<EnvelopeIcon size={16} weight="bold" />
								<span>Verify email</span>
							</button>
						</Tooltip>
					{/if}
					{#if detail.subscription}
						<Tooltip label="Open Stripe billing portal">
							<button
								class="member-detail__action"
								onclick={openPortal}
								disabled={billingBusy}
							>
								<LightningIcon size={16} weight="bold" />
								<span>{billingBusy ? 'Opening…' : 'Billing portal'}</span>
							</button>
						</Tooltip>
					{/if}
					<Tooltip label="Permanently delete this member">
						<button
							class="member-detail__action member-detail__action--danger"
							onclick={deleteMember}
						>
							<TrashIcon size={16} weight="bold" />
							<span>Delete</span>
						</button>
					</Tooltip>
				</div>
			</aside>

			<div class="member-detail__main">
				<!-- Profile section -->
				<section class="member-detail__section">
					<header class="member-detail__section-head">
						<UserCircleIcon size={22} weight="duotone" color="var(--color-teal)" />
						<h2 class="member-detail__h2">Profile</h2>
					</header>
					<form
						class="member-detail__form"
						onsubmit={(e) => {
							e.preventDefault();
							saveProfile();
						}}
					>
						<label class="member-detail__field">
							<span>Name</span>
							<input type="text" bind:value={profileDraft.name} required />
						</label>
						<label class="member-detail__field">
							<span>Email</span>
							<input type="email" bind:value={profileDraft.email} required />
						</label>
						<label class="member-detail__field">
							<span>Phone</span>
							<input
								type="tel"
								bind:value={profileDraft.phone}
								placeholder="+1 555 123 4567"
							/>
						</label>
						<div class="member-detail__field-row">
							<button
								type="submit"
								class="member-detail__btn member-detail__btn--primary"
								disabled={!profileDirty || savingProfile}
							>
								{savingProfile ? 'Saving…' : 'Save profile'}
							</button>
							{#if profileDirty}
								<button
									type="button"
									class="member-detail__btn"
									onclick={() => syncDrafts(u)}
								>
									Discard
								</button>
							{/if}
						</div>
					</form>
				</section>

				<!-- Billing address section -->
				<section class="member-detail__section">
					<header class="member-detail__section-head">
						<MapPinIcon size={22} weight="duotone" color="var(--color-teal)" />
						<h2 class="member-detail__h2">Billing address</h2>
					</header>
					<form
						class="member-detail__form"
						onsubmit={(e) => {
							e.preventDefault();
							saveAddress();
						}}
					>
						<label class="member-detail__field">
							<span>Address line 1</span>
							<input type="text" bind:value={addressDraft.line1} />
						</label>
						<label class="member-detail__field">
							<span>Address line 2</span>
							<input type="text" bind:value={addressDraft.line2} />
						</label>
						<div class="member-detail__field-grid">
							<label class="member-detail__field">
								<span>City</span>
								<input type="text" bind:value={addressDraft.city} />
							</label>
							<label class="member-detail__field">
								<span>State / Region</span>
								<input type="text" bind:value={addressDraft.state} />
							</label>
							<label class="member-detail__field">
								<span>Postal code</span>
								<input type="text" bind:value={addressDraft.postal_code} />
							</label>
							<label class="member-detail__field">
								<span>Country (ISO)</span>
								<input
									type="text"
									maxlength="2"
									placeholder="US"
									bind:value={addressDraft.country}
								/>
							</label>
						</div>
						<div class="member-detail__field-row">
							<button
								type="submit"
								class="member-detail__btn member-detail__btn--primary"
								disabled={!addressDirty || savingAddress}
							>
								{savingAddress ? 'Saving…' : 'Save address'}
							</button>
							{#if addressDirty}
								<button
									type="button"
									class="member-detail__btn"
									onclick={() => syncDrafts(u)}
								>
									Discard
								</button>
							{/if}
						</div>
						<p class="member-detail__hint">
							Changes are written to the local row immediately and pushed to Stripe in
							the background.
						</p>
					</form>
				</section>

				<!-- Subscription section -->
				{#if detail.subscription}
					{@const sub = detail.subscription}
					<section class="member-detail__section">
						<header class="member-detail__section-head">
							<LightningIcon size={22} weight="duotone" color="var(--color-teal)" />
							<h2 class="member-detail__h2">Subscription</h2>
						</header>
						<dl class="member-detail__sub-grid">
							<div>
								<dt>Plan</dt>
								<dd>{planLabel(sub.plan)}</dd>
							</div>
							<div>
								<dt>Status</dt>
								<dd>{sub.status}</dd>
							</div>
							<div>
								<dt>Current period</dt>
								<dd>
									{formatDate(sub.current_period_start)} – {formatDate(
										sub.current_period_end
									)}
								</dd>
							</div>
							<div>
								<dt>Stripe ID</dt>
								<dd class="member-detail__mono">{sub.stripe_subscription_id}</dd>
							</div>
						</dl>
					</section>
				{/if}

				<!-- Activity timeline -->
				<section class="member-detail__section">
					<header class="member-detail__section-head">
						<ShieldCheckIcon size={22} weight="duotone" color="var(--color-teal)" />
						<h2 class="member-detail__h2">Recent activity</h2>
					</header>
					{#if detail.activity.length === 0}
						<p class="member-detail__none">No admin actions recorded yet.</p>
					{:else}
						<ol class="member-detail__timeline">
							{#each detail.activity as entry, i (i)}
								<li class="member-detail__timeline-row">
									<span class="member-detail__timeline-action"
										>{actionLabel(entry.action)}</span
									>
									<span class="member-detail__timeline-time"
										>{formatDateTime(entry.created_at)}</span
									>
								</li>
							{/each}
						</ol>
					{/if}
				</section>

				<!-- Payment failures -->
				{#if detail.payment_failures.length > 0}
					<section class="member-detail__section">
						<header class="member-detail__section-head">
							<LightningIcon size={22} weight="duotone" color="#ef4444" />
							<h2 class="member-detail__h2">Recent payment failures</h2>
						</header>
						<table class="member-detail__pf-table">
							<thead>
								<tr>
									<th>Date</th>
									<th>Amount</th>
									<th>Reason</th>
									<th>Attempt</th>
								</tr>
							</thead>
							<tbody>
								{#each detail.payment_failures as pf, i (i)}
									<tr>
										<td>{formatDateTime(pf.created_at)}</td>
										<td>{formatCents(pf.amount_cents, pf.currency)}</td>
										<td>{pf.failure_message ?? pf.failure_code ?? '—'}</td>
										<td>{pf.attempt_count}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</section>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.member-detail {
		max-width: 72rem;
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

	.member-detail__loading,
	.member-detail__error {
		color: var(--color-grey-400);
	}

	.member-detail__error {
		color: #f87171;
	}

	.member-detail__layout {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1.25rem;
	}

	.member-detail__sidebar {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.member-detail__id {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 1.5rem 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		text-align: center;
	}

	.member-detail__avatar {
		width: 4.5rem;
		height: 4.5rem;
		border-radius: var(--radius-full);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, var(--color-teal-dark), var(--color-deep-blue));
		color: var(--color-white);
		font-size: 1.75rem;
		font-weight: 700;
		text-transform: uppercase;
	}

	.member-detail__name {
		margin: 0;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.member-detail__email {
		margin: 0;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		word-break: break-all;
	}

	.member-detail__pills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		justify-content: center;
		margin-top: 0.4rem;
	}

	.member-detail__pill {
		font-size: 0.6875rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
	}

	.member-detail__pill--active {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}

	.member-detail__pill--suspended {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.member-detail__pill--banned {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.member-detail__role {
		font-size: 0.6875rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		text-transform: capitalize;
	}

	.member-detail__role--admin {
		background-color: rgba(245, 158, 11, 0.12);
		color: #f59e0b;
	}

	.member-detail__role--member {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}

	.member-detail__reason {
		margin: 0.5rem 0 0;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		line-height: 1.4;
	}

	.member-detail__meta {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
		padding: 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		margin: 0;
	}

	.member-detail__meta dt {
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.member-detail__meta dd {
		margin: 0.2rem 0 0;
		font-size: var(--fs-xs);
		color: var(--color-grey-200);
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
	}

	.member-detail__sidebar-actions {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.member-detail__action {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		justify-content: flex-start;
		min-height: 3rem;
		padding: 0 1.25rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: 0.8125rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 150ms var(--ease-out);
		width: 100%;
	}

	.member-detail__action:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.09);
		border-color: rgba(255, 255, 255, 0.16);
	}

	.member-detail__action:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.member-detail__action--warn {
		color: #f59e0b;
	}

	.member-detail__action--danger {
		color: #ef4444;
	}

	.member-detail__main {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.member-detail__section {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.25rem;
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
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.member-detail__form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.member-detail__field {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.member-detail__field span {
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.member-detail__field input {
		min-height: 3rem;
		padding: 0.5rem 0.75rem;
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		color: var(--color-white);
		font-size: 0.875rem;
		outline: none;
		transition:
			border-color 150ms,
			box-shadow 150ms;
	}

	.member-detail__field input:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.member-detail__field-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.625rem;
	}

	.member-detail__field-row {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}

	.member-detail__btn {
		padding: 0.55rem 0.95rem;
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

	.member-detail__hint {
		margin: 0;
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		line-height: 1.4;
	}

	.member-detail__sub-grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.65rem;
		margin: 0;
	}

	.member-detail__sub-grid dt {
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
		margin-bottom: 0.15rem;
	}

	.member-detail__sub-grid dd {
		margin: 0;
		font-size: var(--fs-xs);
		color: var(--color-grey-200);
	}

	.member-detail__mono {
		font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
		word-break: break-all;
	}

	.member-detail__none {
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
		margin: 0;
	}

	.member-detail__timeline {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.member-detail__timeline-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 0.75rem;
		background: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius-md);
	}

	.member-detail__timeline-action {
		font-size: var(--fs-xs);
		font-weight: 600;
		color: var(--color-white);
	}

	.member-detail__timeline-time {
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
	}

	.member-detail__pf-table {
		width: 100%;
		border-collapse: collapse;
	}

	.member-detail__pf-table th {
		text-align: left;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.6rem 0.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.member-detail__pf-table td {
		font-size: var(--fs-xs);
		color: var(--color-grey-200);
		padding: 0.6rem 0.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	@media (min-width: 960px) {
		.member-detail__layout {
			grid-template-columns: 18rem minmax(0, 1fr);
			gap: 1.5rem;
		}

		.member-detail__sidebar {
			position: sticky;
			top: 1.25rem;
			align-self: start;
			max-height: calc(100vh - 2.5rem);
			overflow-y: auto;
		}

		.member-detail__sub-grid {
			grid-template-columns: 1fr 1fr;
			gap: 0.875rem;
		}
	}
</style>
