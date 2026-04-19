<script lang="ts">
	import CreditCard from 'phosphor-svelte/lib/CreditCard';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import Gift from 'phosphor-svelte/lib/Gift';
	import Clock from 'phosphor-svelte/lib/Clock';
	import CalendarBlank from 'phosphor-svelte/lib/CalendarBlank';
	import { ApiError } from '$lib/api/client';
	import {
		adminSubs,
		type CompGrantRequest,
		type UserSubscriptionView
	} from '$lib/api/admin-subs';

	let lookupId = $state('');
	let view = $state<UserSubscriptionView | null>(null);
	let loading = $state(false);
	let error = $state('');
	let toast = $state('');

	let compPlanId = $state('');
	let compDays = $state<number | undefined>(undefined);
	let compNotes = $state('');
	let compBusy = $state(false);

	let extendDays = $state(7);
	let extendNotes = $state('');
	let extendBusy = $state(false);

	let cycleAnchor = $state('');
	let cycleNotes = $state('');
	let cycleBusy = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function lookup(e: Event) {
		e.preventDefault();
		if (!lookupId.trim()) return;
		loading = true;
		error = '';
		view = null;
		try {
			view = await adminSubs.byUser(lookupId.trim());
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Lookup failed';
		} finally {
			loading = false;
		}
	}

	async function compGrant(e: Event) {
		e.preventDefault();
		if (!view || !compPlanId.trim()) return;
		compBusy = true;
		error = '';
		try {
			const req: CompGrantRequest = {
				user_id: lookupId.trim(),
				plan_id: compPlanId.trim(),
				duration_days: compDays ? Number(compDays) : undefined,
				notes: compNotes.trim() || undefined
			};
			const r = await adminSubs.comp(req);
			flash(`Comp membership minted (id ${r.membership_id.slice(0, 8)}…)`);
			compPlanId = '';
			compNotes = '';
			compDays = undefined;
			view = await adminSubs.byUser(lookupId.trim());
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Comp grant failed';
		} finally {
			compBusy = false;
		}
	}

	async function extend(e: Event) {
		e.preventDefault();
		if (!view?.subscription.subscription) return;
		extendBusy = true;
		error = '';
		try {
			const r = await adminSubs.extend(view.subscription.subscription.id, {
				days: Number(extendDays),
				notes: extendNotes.trim() || undefined
			});
			flash(
				`Period end shifted to ${new Date(r.new_current_period_end).toLocaleDateString()}`
			);
			extendNotes = '';
			view = await adminSubs.byUser(lookupId.trim());
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Extension failed';
		} finally {
			extendBusy = false;
		}
	}

	async function overrideCycle(e: Event) {
		e.preventDefault();
		if (!view?.subscription.subscription || !cycleAnchor) return;
		cycleBusy = true;
		error = '';
		try {
			const r = await adminSubs.overrideCycle(view.subscription.subscription.id, {
				anchor: new Date(cycleAnchor).toISOString(),
				notes: cycleNotes.trim() || undefined
			});
			flash(`Anchor moved to ${new Date(r.new_anchor).toLocaleString()}`);
			cycleAnchor = '';
			cycleNotes = '';
			view = await adminSubs.byUser(lookupId.trim());
		} catch (e) {
			error =
				e instanceof ApiError
					? `${e.status}: ${e.message}`
					: 'Billing cycle override failed';
		} finally {
			cycleBusy = false;
		}
	}
</script>

<svelte:head>
	<title>Manual subscriptions · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-subs-manual-page">
	<header class="page__header">
		<div class="page__title-row">
			<CreditCard size={28} weight="duotone" />
			<h1 class="page__title">Manual subscriptions</h1>
		</div>
		<p class="page__subtitle">
			Operator-grade workflows: <strong>comp</strong> a membership without Stripe,
			<strong>extend</strong> the current paid period, or <strong>override</strong> the
			billing-cycle anchor. Every mutation is recorded in <code>subscription_changes</code>
			and the global audit log.
		</p>
	</header>

	{#if toast}<div class="toast">{toast}</div>{/if}
	{#if error}<div class="error" role="alert">{error}</div>{/if}

	<form class="lookup card" onsubmit={lookup}>
		<div class="field field--wide">
			<label class="field__label" for="sub-lookup">Member id (UUID)</label>
			<div class="search-input">
				<MagnifyingGlass size={16} />
				<input
					id="sub-lookup"
					class="field__input"
					placeholder="00000000-…"
					bind:value={lookupId}
					required
					data-testid="sub-lookup-input"
				/>
			</div>
		</div>
		<button class="btn btn--primary" type="submit" disabled={loading}>
			{loading ? 'Loading…' : 'Look up'}
		</button>
	</form>

	{#if view}
		<section class="card">
			<h2 class="card__title">Subscription</h2>
			{#if view.subscription.subscription}
				{@const sub = view.subscription.subscription}
				<dl class="kv">
					<dt>Id</dt>
					<dd><code>{sub.id}</code></dd>
					<dt>Status</dt>
					<dd>
						<span class="badge {view.subscription.is_active ? 'badge--ok' : 'badge--off'}">
							{sub.status}
						</span>
					</dd>
					<dt>Current period</dt>
					<dd>
						{sub.current_period_start
							? new Date(sub.current_period_start).toLocaleDateString()
							: '—'}
						→
						{sub.current_period_end
							? new Date(sub.current_period_end).toLocaleDateString()
							: '—'}
					</dd>
					<dt>Cycle anchor</dt>
					<dd>
						{sub.billing_cycle_anchor
							? new Date(sub.billing_cycle_anchor).toLocaleString()
							: '—'}
					</dd>
					<dt>Stripe</dt>
					<dd>
						{sub.stripe_subscription_id ?? '—'}
						{#if sub.stripe_customer_id}
							·&nbsp;<code>{sub.stripe_customer_id}</code>
						{/if}
					</dd>
				</dl>

				<div class="ops">
					<div class="op">
						<h3 class="op__title">
							<Clock size={18} weight="duotone" />
							Extend period
						</h3>
						<form class="op__form" onsubmit={extend}>
							<input
								class="field__input"
								type="number"
								min="1"
								max="366"
								bind:value={extendDays}
								required
							/>
							<input
								class="field__input"
								placeholder="optional notes"
								bind:value={extendNotes}
							/>
							<button
								class="btn btn--primary"
								type="submit"
								disabled={extendBusy}
								data-testid="sub-extend"
							>
								{extendBusy ? 'Extending…' : `Extend ${extendDays} days`}
							</button>
						</form>
					</div>

					<div class="op">
						<h3 class="op__title">
							<CalendarBlank size={18} weight="duotone" />
							Override cycle anchor
						</h3>
						<form class="op__form" onsubmit={overrideCycle}>
							<input
								class="field__input"
								type="datetime-local"
								bind:value={cycleAnchor}
								required
							/>
							<input
								class="field__input"
								placeholder="optional notes"
								bind:value={cycleNotes}
							/>
							<button
								class="btn btn--primary"
								type="submit"
								disabled={cycleBusy}
								data-testid="sub-cycle"
							>
								{cycleBusy ? 'Updating…' : 'Apply anchor'}
							</button>
						</form>
					</div>
				</div>
			{:else}
				<p class="muted">No active Stripe subscription for this user.</p>
			{/if}
		</section>

		<section class="card">
			<h2 class="card__title">Memberships</h2>
			{#if view.memberships.length === 0}
				<p class="muted">No active memberships.</p>
			{:else}
				<table class="table">
					<thead>
						<tr>
							<th>Plan</th>
							<th>Status</th>
							<th>Starts</th>
							<th>Ends</th>
							<th>Granted by</th>
						</tr>
					</thead>
					<tbody>
						{#each view.memberships as m (m.id)}
							<tr>
								<td><code>{m.plan_id.slice(0, 8)}…</code></td>
								<td>
									<span class="badge badge--ok">{m.status}</span>
								</td>
								<td>{new Date(m.starts_at).toLocaleDateString()}</td>
								<td>{m.ends_at ? new Date(m.ends_at).toLocaleDateString() : '∞'}</td>
								<td>{m.granted_by}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</section>

		<section class="card">
			<h2 class="card__title">
				<Gift size={20} weight="duotone" />
				Comp / gift membership
			</h2>
			<form class="comp-form" onsubmit={compGrant}>
				<div class="field">
					<label class="field__label" for="comp-plan">Plan id (UUID)</label>
					<input
						id="comp-plan"
						class="field__input"
						placeholder="membership_plan id"
						bind:value={compPlanId}
						required
						data-testid="sub-comp-plan"
					/>
				</div>
				<div class="field field--narrow">
					<label class="field__label" for="comp-days">Duration (days)</label>
					<input
						id="comp-days"
						class="field__input"
						type="number"
						min="1"
						max="3650"
						placeholder="leave blank = open-ended"
						bind:value={compDays}
					/>
				</div>
				<div class="field field--wide">
					<label class="field__label" for="comp-notes">Notes (audit)</label>
					<input
						id="comp-notes"
						class="field__input"
						placeholder="ticket #1234 — VIP comp"
						bind:value={compNotes}
					/>
				</div>
				<button
					class="btn btn--primary"
					type="submit"
					disabled={compBusy}
					data-testid="sub-comp-submit"
				>
					{compBusy ? 'Granting…' : 'Grant membership'}
				</button>
			</form>
		</section>
	{/if}
</div>

<style>
	.page {
		max-width: 1100px;
	}
	.page__header {
		margin-bottom: var(--space-5);
	}
	.page__title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-white);
	}
	.page__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		margin: 0;
	}
	.page__subtitle {
		margin-top: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		max-width: 75ch;
	}
	.toast {
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.lookup {
		display: flex;
		align-items: end;
		gap: var(--space-3);
		margin-bottom: var(--space-4);
	}
	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-5);
		margin-bottom: var(--space-4);
	}
	.card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0 0 var(--space-3) 0;
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field--wide {
		flex: 1;
	}
	.field--narrow {
		max-width: 12rem;
	}
	.field__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
	}
	.field__input {
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		width: 100%;
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.search-input {
		position: relative;
	}
	.search-input :global(svg) {
		position: absolute;
		left: 0.7rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-400);
		pointer-events: none;
	}
	.search-input .field__input {
		padding-left: 2rem;
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
	}
	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.btn--primary {
		background: var(--color-teal);
		color: var(--color-white);
	}
	.kv {
		display: grid;
		grid-template-columns: 9rem 1fr;
		gap: var(--space-2) var(--space-3);
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
	}
	.kv dt {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.kv dd {
		margin: 0;
	}
	.badge {
		display: inline-block;
		padding: 0.1rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.badge--ok {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}
	.badge--off {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
	.ops {
		display: grid;
		grid-template-columns: 1fr;
		gap: var(--space-3);
		margin-top: var(--space-4);
	}
	@media (min-width: 880px) {
		.ops {
			grid-template-columns: 1fr 1fr;
		}
	}
	.op {
		padding: var(--space-3);
		background: rgba(0, 0, 0, 0.15);
		border-radius: var(--radius-lg);
	}
	.op__title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0 0 var(--space-2) 0;
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.op__form {
		display: grid;
		grid-template-columns: 1fr 1.5fr auto;
		gap: var(--space-2);
	}
	.comp-form {
		display: grid;
		grid-template-columns: 1.5fr 1fr 2fr auto;
		gap: var(--space-3);
		align-items: end;
	}
	.table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.table th {
		text-align: left;
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		padding: var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}
	.table td {
		padding: var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}
</style>
