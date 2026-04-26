<script lang="ts">
	import { onMount } from 'svelte';
	import IdentificationBadgeIcon from 'phosphor-svelte/lib/IdentificationBadgeIcon';
	import ArrowClockwiseIcon from 'phosphor-svelte/lib/ArrowClockwiseIcon';
	import StopCircleIcon from 'phosphor-svelte/lib/StopCircleIcon';
	import { ApiError } from '$lib/api/client';
	import { impersonation, type ImpersonationSession } from '$lib/api/admin-security';

	let sessions = $state<ImpersonationSession[]>([]);
	let nextCursor = $state<string | null>(null);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let mintTarget = $state('');
	let mintReason = $state('');
	let mintTtl = $state('30');
	let minting = $state(false);
	let mintResult = $state<{ token: string; expires_at: string } | null>(null);

	let revokingId = $state<string | null>(null);
	let revokeReason = $state('');

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			const res = await impersonation.list();
			sessions = res.data;
			nextCursor = res.next_cursor ?? null;
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load sessions';
		} finally {
			loading = false;
		}
	}

	async function loadMore() {
		if (!nextCursor) return;
		try {
			const res = await impersonation.list(nextCursor);
			sessions = [...sessions, ...res.data];
			nextCursor = res.next_cursor ?? null;
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load page';
		}
	}

	async function mint(e: Event) {
		e.preventDefault();
		if (!mintTarget.trim() || !mintReason.trim()) return;
		minting = true;
		mintResult = null;
		error = '';
		try {
			const ttl = Math.max(1, Math.min(120, parseInt(mintTtl, 10) || 30));
			const res = await impersonation.mint({
				target_user_id: mintTarget.trim(),
				reason: mintReason.trim(),
				ttl_minutes: ttl
			});
			mintResult = { token: res.access_token, expires_at: res.expires_at };
			mintTarget = '';
			mintReason = '';
			flash('Impersonation token minted');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to mint token';
		} finally {
			minting = false;
		}
	}

	async function confirmRevoke(id: string) {
		if (!revokeReason.trim()) {
			error = 'Revoke reason required';
			return;
		}
		try {
			await impersonation.revoke(id, revokeReason.trim());
			flash('Session revoked');
			revokingId = null;
			revokeReason = '';
			await refresh();
		} catch (e) {
			error =
				e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to revoke session';
		}
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Impersonation · Security · Admin</title>
</svelte:head>

<div class="page" data-testid="security-impersonation">
	<header class="page__header">
		<a href="/admin/security" class="page__back">← Security</a>
		<div class="page__title-row">
			<IdentificationBadgeIcon size={26} weight="duotone" />
			<h1 class="page__title">Impersonation sessions</h1>
		</div>
		<p class="page__subtitle">
			Mint short-lived tokens to act on behalf of a member. The subject is e-mailed the moment
			a session starts (GDPR Art. 32). Every downstream admin action is tagged with the
			impersonation session id in the audit log.
		</p>
	</header>

	{#if toast}
		<div class="toast">{toast}</div>
	{/if}
	{#if error}
		<div class="error" role="alert" data-testid="impersonation-error">{error}</div>
	{/if}

	<section class="card">
		<h2 class="card__title">Mint new session</h2>
		<form class="form form--mint" onsubmit={mint}>
			<div class="field">
				<label class="field__label" for="imp-target">Target user id (UUID)</label>
				<input
					id="imp-target"
					data-testid="imp-target-input"
					class="field__input"
					placeholder="00000000-0000-0000-0000-000000000000"
					bind:value={mintTarget}
					required
				/>
			</div>
			<div class="field">
				<label class="field__label" for="imp-reason">Reason (≥ 10 chars)</label>
				<input
					id="imp-reason"
					data-testid="imp-reason-input"
					class="field__input"
					placeholder="ticket #1234 — investigating failed checkout"
					bind:value={mintReason}
					minlength="10"
					required
				/>
			</div>
			<div class="field field--narrow">
				<label class="field__label" for="imp-ttl">TTL (min, 1–120)</label>
				<input
					id="imp-ttl"
					class="field__input"
					type="number"
					min="1"
					max="120"
					bind:value={mintTtl}
				/>
			</div>
			<button
				class="btn btn--primary"
				type="submit"
				data-testid="imp-mint-button"
				disabled={minting}
			>
				{minting ? 'Minting…' : 'Mint token'}
			</button>
		</form>

		{#if mintResult}
			<div class="mint-result" data-testid="imp-mint-result">
				<p class="mint-result__label">Access token (copy now — not shown again)</p>
				<code class="mint-result__token">{mintResult.token}</code>
				<p class="mint-result__expires">
					Expires {new Date(mintResult.expires_at).toLocaleString()}
				</p>
			</div>
		{/if}
	</section>

	<section class="card">
		<header class="card__heading">
			<h2 class="card__title">Recent sessions</h2>
			<button class="btn btn--ghost" onclick={refresh}>
				<ArrowClockwiseIcon size={16} weight="bold" />
				Refresh
			</button>
		</header>

		{#if loading}
			<p class="muted">Loading…</p>
		{:else if sessions.length === 0}
			<p class="muted">No impersonation sessions on record.</p>
		{:else}
			<table class="table" data-testid="imp-sessions-table">
				<thead>
					<tr>
						<th>Session</th>
						<th>Target</th>
						<th>Reason</th>
						<th>Status</th>
						<th>Expires</th>
						<th aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each sessions as session (session.id)}
						<tr>
							<td><code title={session.id}>{session.id.slice(0, 8)}…</code></td>
							<td
								><code title={session.target_user_id}
									>{session.target_user_id.slice(0, 8)}…</code
								></td
							>
							<td class="reason-cell">{session.reason}</td>
							<td>
								<span
									class="badge"
									class:badge--ok={!session.revoked_at &&
										new Date(session.expires_at) > new Date()}
									class:badge--off={!!session.revoked_at ||
										new Date(session.expires_at) <= new Date()}
								>
									{session.revoked_at
										? 'Revoked'
										: new Date(session.expires_at) <= new Date()
											? 'Expired'
											: 'Active'}
								</span>
							</td>
							<td>{new Date(session.expires_at).toLocaleString()}</td>
							<td class="row-actions">
								{#if !session.revoked_at}
									<button
										class="btn btn--danger"
										onclick={() => {
											revokingId = session.id;
											revokeReason = '';
										}}
										aria-label="Revoke session"
									>
										<StopCircleIcon size={16} weight="bold" />
										Revoke
									</button>
								{/if}
							</td>
						</tr>

						{#if revokingId === session.id}
							<tr>
								<td colspan="6">
									<div class="revoke-row">
										<input
											class="field__input"
											placeholder="Reason for revoking…"
											bind:value={revokeReason}
										/>
										<button
											class="btn btn--danger"
											onclick={() => confirmRevoke(session.id)}
											data-testid="imp-revoke-confirm"
										>
											Confirm revoke
										</button>
										<button
											class="btn btn--ghost"
											onclick={() => {
												revokingId = null;
												revokeReason = '';
											}}
										>
											Cancel
										</button>
									</div>
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>

			{#if nextCursor}
				<button class="btn btn--ghost load-more" onclick={loadMore}>Load more</button>
			{/if}
		{/if}
	</section>
</div>

<style>
	.page {
		max-width: 1100px;
	}
	.page__header {
		margin-bottom: var(--space-6);
	}
	.page__back {
		display: inline-block;
		margin-bottom: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-teal);
		text-decoration: none;
	}
	.page__back:hover {
		text-decoration: underline;
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
		max-width: 65ch;
	}
	.toast {
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.card {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: var(--space-5);
		margin-bottom: var(--space-5);
	}
	.card__heading {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}
	.card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0 0 var(--space-3);
	}
	.form {
		display: grid;
		gap: var(--space-3);
	}
	.form--mint {
		grid-template-columns: 2fr 3fr 1fr auto;
		align-items: end;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field--narrow {
		max-width: 8rem;
	}
	.field__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
		font-weight: var(--w-medium);
	}
	.field__input {
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn--primary {
		background: var(--color-teal);
		color: var(--color-white);
	}
	.btn--primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn--ghost {
		border-color: rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
	}
	.btn--ghost:hover {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}
	.btn--danger {
		background: rgba(239, 68, 68, 0.1);
		color: #fca5a5;
		border-color: rgba(239, 68, 68, 0.25);
	}
	.btn--danger:hover {
		background: rgba(239, 68, 68, 0.18);
	}
	.mint-result {
		margin-top: var(--space-4);
		padding: var(--space-4);
		border: 1px dashed rgba(15, 164, 175, 0.4);
		border-radius: var(--radius-2xl);
		background: rgba(15, 164, 175, 0.06);
	}
	.mint-result__label {
		font-size: var(--fs-xs);
		color: var(--color-teal-light);
		margin: 0 0 var(--space-2);
		font-weight: var(--w-bold);
	}
	.mint-result__token {
		display: block;
		padding: var(--space-2);
		background: rgba(0, 0, 0, 0.3);
		border-radius: var(--radius-md);
		font-size: var(--fs-xs);
		word-break: break-all;
		color: var(--color-white);
	}
	.mint-result__expires {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin: var(--space-2) 0 0;
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
		padding: var(--space-3) var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}
	.reason-cell {
		max-width: 30ch;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.row-actions {
		display: flex;
		gap: var(--space-2);
		justify-content: flex-end;
	}
	.revoke-row {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.revoke-row .field__input {
		flex: 1;
	}
	.badge {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
	}
	.badge--ok {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}
	.badge--off {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
	.load-more {
		margin-top: var(--space-3);
	}
</style>
