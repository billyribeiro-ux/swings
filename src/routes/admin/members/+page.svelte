<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import type { UserResponse, PaginatedResponse } from '$lib/api/types';
	import ActionMenu from '$lib/components/shared/ActionMenu.svelte';
	import ActionMenuItem from '$lib/components/shared/ActionMenuItem.svelte';
	import ActionMenuDivider from '$lib/components/shared/ActionMenuDivider.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import UserIcon from 'phosphor-svelte/lib/UserIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import ProhibitIcon from 'phosphor-svelte/lib/ProhibitIcon';
	import ClockCountdownIcon from 'phosphor-svelte/lib/ClockCountdownIcon';
	import DotsThreeVerticalIcon from 'phosphor-svelte/lib/DotsThreeVerticalIcon';

	type StatusFilter = '' | 'active' | 'suspended' | 'banned' | 'unverified';

	let members = $state<UserResponse[]>([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let search = $state('');
	let roleFilter = $state<'' | 'admin' | 'member'>('');
	let statusFilter = $state<StatusFilter>('');
	let searchTimeout: ReturnType<typeof setTimeout>;
	/** Monotonic guard so slower network responses cannot overwrite newer search/filter results. */
	let membersLoadGeneration = 0;

	async function loadMembers() {
		const gen = ++membersLoadGeneration;
		loading = true;
		try {
			// We hand-build the query string instead of allocating a
			// `URLSearchParams` because `svelte/prefer-svelte-reactivity`
			// flags the mutable instance and a non-reactive plain string
			// is what we actually want here (the request fires once per
			// `loadMembers()`).
			const q = search.trim();
			const parts: string[] = [`page=${page}`, `per_page=15`];
			if (q) parts.push(`search=${encodeURIComponent(q)}`);
			if (roleFilter) parts.push(`role=${roleFilter}`);
			if (statusFilter) parts.push(`status=${statusFilter}`);
			const url = `/api/admin/members?${parts.join('&')}`;
			const res = await api.get<PaginatedResponse<UserResponse>>(url);
			if (gen !== membersLoadGeneration) return;
			members = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch (e) {
			if (gen !== membersLoadGeneration) return;
			toast.error(e instanceof ApiError ? e.message : 'Failed to load members');
		} finally {
			if (gen === membersLoadGeneration) loading = false;
		}
	}

	onMount(loadMembers);

	function scheduleSearchReload() {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			page = 1;
			loadMembers();
		}, 280);
	}

	/** Run search immediately (Enter in the field, search form submit). */
	function commitSearchNow(e: Event) {
		e.preventDefault();
		clearTimeout(searchTimeout);
		page = 1;
		loadMembers();
	}

	onDestroy(() => clearTimeout(searchTimeout));

	function clearMemberFilters() {
		clearTimeout(searchTimeout);
		search = '';
		roleFilter = '';
		statusFilter = '';
		page = 1;
		loadMembers();
	}

	const hasMemberFilters = $derived(
		Boolean(search.trim() || roleFilter || statusFilter)
	);

	function changeRole(r: typeof roleFilter) {
		roleFilter = r;
		page = 1;
		loadMembers();
	}

	function changeStatus(s: StatusFilter) {
		statusFilter = s;
		page = 1;
		loadMembers();
	}

	type Lifecycle = 'banned' | 'suspended' | 'active';
	function lifecycle(m: UserResponse): Lifecycle {
		if (m.banned_at) return 'banned';
		if (m.suspended_at) return 'suspended';
		return 'active';
	}

	function statusLabel(state: Lifecycle): string {
		return state === 'banned' ? 'Banned' : state === 'suspended' ? 'Suspended' : 'Active';
	}

	async function viewMember(member: UserResponse) {
		await goto(resolve('/admin/members/[id]', { id: member.id }));
	}

	async function banOrUnban(member: UserResponse) {
		if (member.banned_at) {
			const ok = await confirmDialog({
				title: 'Lift ban?',
				message: `Reinstate ${member.name}'s account access?`,
				confirmLabel: 'Lift ban',
				variant: 'warning'
			});
			if (!ok) return;
			try {
				await api.post(`/api/admin/members/${member.id}/unban`, {});
				toast.success('Ban lifted');
				await loadMembers();
			} catch (e) {
				toast.error(e instanceof ApiError ? e.message : 'Failed to lift ban');
			}
			return;
		}
		const reason = window.prompt(`Reason for banning ${member.name}? (optional)`) ?? '';
		const ok = await confirmDialog({
			title: 'Ban member?',
			message: `Banning ${member.name} revokes their session and immediately cancels any active subscription.`,
			confirmLabel: 'Ban member',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.post(`/api/admin/members/${member.id}/ban`, { reason });
			toast.success('Member banned');
			await loadMembers();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Ban failed');
		}
	}

	async function suspendOrUnsuspend(member: UserResponse) {
		if (member.suspended_at) {
			const ok = await confirmDialog({
				title: 'Lift suspension?',
				message: `Restore ${member.name}'s ability to log in?`,
				confirmLabel: 'Lift suspension',
				variant: 'warning'
			});
			if (!ok) return;
			try {
				await api.post(`/api/admin/members/${member.id}/unsuspend`, {});
				toast.success('Suspension lifted');
				await loadMembers();
			} catch (e) {
				toast.error(e instanceof ApiError ? e.message : 'Failed to lift suspension');
			}
			return;
		}
		const reason = window.prompt(`Reason for suspending ${member.name}? (optional)`) ?? '';
		const days = window.prompt('Suspension duration in days (blank = open-ended):', '');
		let until: string | undefined;
		if (days && Number.parseInt(days, 10) > 0) {
			const ts = new Date(Date.now() + Number.parseInt(days, 10) * 86400000);
			until = ts.toISOString();
		}
		try {
			await api.post(`/api/admin/members/${member.id}/suspend`, { reason, until });
			toast.success(
				until
					? `Suspended until ${new Date(until).toLocaleDateString()}`
					: 'Member suspended'
			);
			await loadMembers();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Suspension failed');
		}
	}

	async function deleteMember(member: UserResponse) {
		const ok = await confirmDialog({
			title: 'Delete member?',
			message: `Permanently remove ${member.name}? This cancels any active Stripe subscription and cannot be undone.`,
			confirmLabel: 'Delete',
			cancelLabel: 'Keep account',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.del(`/api/admin/members/${member.id}`);
			toast.success('Member deleted');
			await loadMembers();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Delete failed');
		}
	}

	async function quickEdit(member: UserResponse) {
		await goto(resolve('/admin/members/[id]', { id: member.id }));
	}

	async function toggleRole(member: UserResponse) {
		const newRole = member.role === 'admin' ? 'member' : 'admin';
		const ok = await confirmDialog({
			title: 'Change role?',
			message:
				newRole === 'admin'
					? `Promote ${member.name} to administrator? They will have full operator access.`
					: `Demote ${member.name} to member? They will lose administrator access.`,
			confirmLabel: 'Change role',
			variant: 'warning'
		});
		if (!ok) return;
		try {
			await api.put(`/api/admin/members/${member.id}/role`, { role: newRole });
			toast.success('Role updated');
			await loadMembers();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to update role');
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Members - Admin - Precision Options Signals</title>
</svelte:head>

<div class="members-page">
	<header class="members-page__page-header">
		<div class="members-page__heading">
			<h1 class="members-page__title">Members</h1>
			<p class="members-page__subtitle">
				{total.toLocaleString()} total {total === 1 ? 'member' : 'members'} — manage roles, lifecycle,
				and billing profile. To add someone new, use <strong>Search &amp; create</strong> (role, temp
				password, optional setup email).
			</p>
		</div>
		<a class="members-page__cta" href={resolve('/admin/members/manage')}>
			<MagnifyingGlassIcon size={16} weight="bold" />
			<span>Search &amp; create</span>
			<ArrowRightIcon size={14} weight="bold" />
		</a>
	</header>

	<!-- Filter / search bar -->
	<div class="members-page__toolbar">
		<form
			class="members-page__search"
			role="search"
			aria-label="Search members by name or email"
			onsubmit={commitSearchNow}
		>
			<MagnifyingGlassIcon size={16} weight="bold" class="members-page__search-icon" />
			<input
				id="member-search"
				name="search"
				type="search"
				class="members-page__search-input"
				placeholder="Search by name or email…"
				autocomplete="off"
				spellcheck="false"
				enterkeyhint="search"
				bind:value={search}
				oninput={scheduleSearchReload}
			/>
		</form>
		<div class="members-page__tabs" role="tablist" aria-label="Filter by role">
			<button
				class="members-page__tab"
				class:members-page__tab--active={roleFilter === ''}
				onclick={() => changeRole('')}
				role="tab"
				aria-selected={roleFilter === ''}
			>
				All roles
			</button>
			<button
				class="members-page__tab"
				class:members-page__tab--active={roleFilter === 'admin'}
				onclick={() => changeRole('admin')}
				role="tab"
				aria-selected={roleFilter === 'admin'}
			>
				Admins
			</button>
			<button
				class="members-page__tab"
				class:members-page__tab--active={roleFilter === 'member'}
				onclick={() => changeRole('member')}
				role="tab"
				aria-selected={roleFilter === 'member'}
			>
				Members
			</button>
		</div>
		<div class="members-page__tabs" role="tablist" aria-label="Filter by status">
			<button
				class="members-page__tab"
				class:members-page__tab--active={statusFilter === ''}
				onclick={() => changeStatus('')}
				role="tab"
				aria-selected={statusFilter === ''}
			>
				Any status
			</button>
			<button
				class="members-page__tab"
				class:members-page__tab--active={statusFilter === 'active'}
				onclick={() => changeStatus('active')}
				role="tab"
				aria-selected={statusFilter === 'active'}
			>
				Active
			</button>
			<button
				class="members-page__tab"
				class:members-page__tab--active={statusFilter === 'suspended'}
				onclick={() => changeStatus('suspended')}
				role="tab"
				aria-selected={statusFilter === 'suspended'}
			>
				Suspended
			</button>
			<button
				class="members-page__tab"
				class:members-page__tab--active={statusFilter === 'banned'}
				onclick={() => changeStatus('banned')}
				role="tab"
				aria-selected={statusFilter === 'banned'}
			>
				Banned
			</button>
		</div>
	</div>

	{#if loading}
		<div class="members-page__skeleton" aria-hidden="true">
			{#each Array(5) as _, i (i)}
				<div class="members-page__skeleton-row"></div>
			{/each}
		</div>
	{:else if members.length === 0}
		<div class="members-page__empty" role="status" aria-live="polite">
			<div class="members-page__empty-panel">
				<div class="members-page__empty-icon" aria-hidden="true">
					<UsersIcon size={28} weight="duotone" color="var(--color-grey-400)" />
				</div>
				<h2 class="members-page__empty-title">
					{hasMemberFilters ? 'No matching members' : 'No members yet'}
				</h2>
				<p class="members-page__empty-body">
					{hasMemberFilters
						? 'Adjust your search or clear filters to see everyone in the directory.'
						: 'New sign-ups show up here automatically, or add someone from search & create.'}
				</p>
				<div class="members-page__empty-actions">
					{#if hasMemberFilters}
						<button
							type="button"
							class="members-page__empty-btn members-page__empty-btn--primary"
							onclick={clearMemberFilters}
						>
							Clear filters
						</button>
					{/if}
					<a
						href={resolve('/admin/members/manage')}
						class={[
							'members-page__empty-btn',
							hasMemberFilters
								? 'members-page__empty-btn--secondary'
								: 'members-page__empty-btn--primary'
						]}
					>
						{hasMemberFilters ? 'Add a member' : 'Search & create'}
					</a>
				</div>
			</div>
		</div>
	{:else}
		<!-- Mobile: Card view -->
		<div class="members-page__cards">
			{#each members as member (member.id)}
				{@const state = lifecycle(member)}
				<div class="member-card" data-state={state}>
					<div class="member-card__header">
						<div class="member-card__identity">
							<span class="member-card__avatar" aria-hidden="true">
								{member.name?.[0]?.toUpperCase() || '?'}
							</span>
							<div class="member-card__name-block">
								<span class="member-card__name">{member.name}</span>
								<span class="member-card__email">{member.email}</span>
							</div>
						</div>
						<span
							class={[
								'member-card__role',
								member.role === 'admin'
									? 'member-card__role--admin'
									: 'member-card__role--member'
							]}
						>
							{member.role}
						</span>
					</div>
					<div class="member-card__row">
						<span class="member-card__label">Status</span>
						<span class={['member-card__pill', `member-card__pill--${state}`]}
							>{statusLabel(state)}</span
						>
					</div>
					<div class="member-card__row">
						<span class="member-card__label">Joined</span>
						<span class="member-card__value">{formatDate(member.created_at)}</span>
					</div>
					<div class="member-card__actions">
						<a
							href={resolve('/admin/members/[id]', { id: member.id })}
							class="member-card__profile"
						>
							<EyeIcon size={14} weight="bold" />
							<span>View profile</span>
						</a>
						<ActionMenu placement="bottom-end" label="Member actions">
							{#snippet trigger(p)}
								<button
									type="button"
									{...p}
									class="member-card__menu-trigger"
									aria-label="Open member actions menu"
								>
									<DotsThreeVerticalIcon size={18} weight="bold" />
								</button>
							{/snippet}
							{#snippet items()}
								<ActionMenuItem
									icon={UserIcon}
									onclick={() => viewMember(member)}
								>
									View profile
								</ActionMenuItem>
								<ActionMenuItem
									icon={PencilSimpleIcon}
									onclick={() => quickEdit(member)}
								>
									Edit profile
								</ActionMenuItem>
								<ActionMenuItem
									icon={ClockCountdownIcon}
									onclick={() => suspendOrUnsuspend(member)}
								>
									{member.suspended_at ? 'Lift suspension' : 'Suspend sign-in'}
								</ActionMenuItem>
								<ActionMenuItem
									icon={ShieldCheckIcon}
									onclick={() => toggleRole(member)}
								>
									{member.role === 'admin' ? 'Demote to member' : 'Promote to admin'}
								</ActionMenuItem>
								<ActionMenuItem
									icon={ProhibitIcon}
									onclick={() => banOrUnban(member)}
								>
									{member.banned_at ? 'Lift ban' : 'Ban'}
								</ActionMenuItem>
								<ActionMenuDivider />
								<ActionMenuItem
									icon={TrashIcon}
									variant="danger"
									onclick={() => deleteMember(member)}
								>
									Delete account
								</ActionMenuItem>
							{/snippet}
						</ActionMenu>
					</div>
				</div>
			{/each}
		</div>
		<!-- Tablet+: Table view -->
		<div class="members-page__table-wrap">
			<table class="m-table">
				<thead>
					<tr>
						<th>Name</th>
						<th>Email</th>
						<th>Role</th>
						<th>Status</th>
						<th>Joined</th>
						<th class="m-table__th-actions">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each members as member (member.id)}
						{@const state = lifecycle(member)}
						<tr>
							<td>
								<button
									type="button"
									class="m-table__identity m-table__identity--btn"
									onclick={() => viewMember(member)}
								>
									<span class="m-table__avatar" aria-hidden="true">
										{member.name?.[0]?.toUpperCase() || '?'}
									</span>
									<span class="m-table__name">{member.name}</span>
								</button>
							</td>
							<td class="m-table__muted">{member.email}</td>
							<td>
								<span
									class={[
										'm-table__role',
										member.role === 'admin'
											? 'm-table__role--admin'
											: 'm-table__role--member'
									]}
								>
									{member.role}
								</span>
							</td>
							<td>
								<span class={['m-table__pill', `m-table__pill--${state}`]}>
									{statusLabel(state)}
								</span>
							</td>
							<td class="m-table__muted">{formatDate(member.created_at)}</td>
							<td>
								<div class="m-table__actions">
									<ActionMenu placement="bottom-end" label="Member actions">
										{#snippet trigger(p)}
											<button
												type="button"
												{...p}
												class="m-table__menu-trigger"
												aria-label="Open member actions menu"
											>
												<DotsThreeVerticalIcon size={18} weight="bold" />
											</button>
										{/snippet}
										{#snippet items()}
											<ActionMenuItem
												icon={UserIcon}
												onclick={() => viewMember(member)}
											>
												View profile
											</ActionMenuItem>
											<ActionMenuItem
												icon={PencilSimpleIcon}
												onclick={() => quickEdit(member)}
											>
												Edit profile
											</ActionMenuItem>
											<ActionMenuItem
												icon={ClockCountdownIcon}
												onclick={() => suspendOrUnsuspend(member)}
											>
												{member.suspended_at ? 'Lift suspension' : 'Suspend sign-in'}
											</ActionMenuItem>
											<ActionMenuItem
												icon={ShieldCheckIcon}
												onclick={() => toggleRole(member)}
											>
												{member.role === 'admin'
													? 'Demote to member'
													: 'Promote to admin'}
											</ActionMenuItem>
											<ActionMenuItem
												icon={ProhibitIcon}
												onclick={() => banOrUnban(member)}
											>
												{member.banned_at ? 'Lift ban' : 'Ban'}
											</ActionMenuItem>
											<ActionMenuDivider />
											<ActionMenuItem
												icon={TrashIcon}
												variant="danger"
												onclick={() => deleteMember(member)}
											>
												Delete account
											</ActionMenuItem>
										{/snippet}
									</ActionMenu>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<div class="members-page__pagination">
				<button
					onclick={() => {
						page--;
						loadMembers();
					}}
					disabled={page <= 1}
					class="members-page__page-btn"
					aria-label="Previous page"
				>
					<CaretLeftIcon size={16} weight="bold" />
					<span>Prev</span>
				</button>
				<span class="members-page__page-info">Page {page} of {totalPages}</span>
				<button
					onclick={() => {
						page++;
						loadMembers();
					}}
					disabled={page >= totalPages}
					class="members-page__page-btn"
					aria-label="Next page"
				>
					<span>Next</span>
					<CaretRightIcon size={16} weight="bold" />
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	@keyframes shimmer {
		0% {
			background-position: -200% 0;
		}
		100% {
			background-position: 200% 0;
		}
	}

	.members-page__page-header {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		margin-bottom: 1.25rem;
	}

	.members-page__title {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.2;
		letter-spacing: -0.01em;
		margin: 0 0 0.5rem;
	}

	.members-page__subtitle {
		font-size: 0.875rem;
		font-weight: 400;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
		hyphens: none;
	}

	.members-page__cta {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 3rem;
		padding: 0 1.25rem;
		background: rgba(255, 255, 255, 0.05);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		color: var(--color-white);
		font-size: 0.8125rem;
		font-weight: 600;
		text-decoration: none;
		align-self: flex-start;
		transition: all 200ms var(--ease-out);
	}

	.members-page__cta:hover {
		background-color: rgba(255, 255, 255, 0.08);
		border-color: rgba(255, 255, 255, 0.16);
		transform: translateY(-1px);
	}

	.members-page__toolbar {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		margin-bottom: 1rem;
	}

	.members-page__search {
		position: relative;
		display: flex;
		align-items: center;
		margin: 0;
		padding: 0;
		border: none;
	}

	.members-page__search :global(.members-page__search-icon) {
		position: absolute;
		left: 0.875rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	.members-page__search-input {
		width: 100%;
		min-height: 3rem;
		padding: 0 1.25rem 0 2.6rem;
		background: rgba(255, 255, 255, 0.05);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		color: var(--color-white);
		font-size: 0.875rem;
		font-weight: 400;
		outline: none;
		transition:
			border-color 150ms,
			box-shadow 150ms;
	}

	.members-page__search-input::placeholder {
		color: var(--color-grey-500);
	}

	.members-page__search-input:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.members-page__tabs {
		display: inline-flex;
		gap: 0.25rem;
		padding: 0.25rem;
		background: rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		align-self: flex-start;
		flex-wrap: wrap;
	}

	.members-page__tab {
		min-height: 2.75rem;
		padding: 0 1.25rem;
		border: none;
		border-radius: var(--radius-lg);
		background: transparent;
		color: var(--color-grey-400);
		font-size: 0.8125rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}

	.members-page__tab:hover {
		color: var(--color-white);
	}

	.members-page__tab--active {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	.members-page__skeleton {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.members-page__skeleton-row {
		/* CLS guard (mobile, card view): real `.member-card` runs ~5rem
		   tall on a typical viewport — the prior 80px overshoots by
		   ~10px per row × 5 rows = ~50px reflow when the data lands.
		   Pinning to the same `.member-card` height keeps the layout
		   box stable through the loading → ready transition. */
		height: 5rem;
		border-radius: var(--radius-xl);
		background: linear-gradient(
			90deg,
			rgba(255, 255, 255, 0.03) 0%,
			rgba(255, 255, 255, 0.06) 50%,
			rgba(255, 255, 255, 0.03) 100%
		);
		background-size: 200% 100%;
		animation: shimmer 1.6s ease-in-out infinite;
	}

	/* Empty state: centered, width-capped panel (common product pattern — readable measure, not full-bleed). */
	.members-page__empty {
		display: flex;
		justify-content: center;
		align-items: flex-start;
		padding: clamp(2rem, 5vw, 3rem) 1rem;
		min-height: min(17.5rem, 42vh);
	}

	.members-page__empty-panel {
		width: 100%;
		max-width: 21.5rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: 2rem 1.5rem 1.875rem;
		background: rgba(19, 43, 80, 0.45);
		backdrop-filter: blur(20px);
		-webkit-backdrop-filter: blur(20px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.06) inset,
			0 12px 32px -12px rgba(0, 0, 0, 0.35);
	}

	.members-page__empty-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 3.5rem;
		height: 3.5rem;
		margin-bottom: 0.25rem;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.055);
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	.members-page__empty-title {
		font-family: var(--font-heading);
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--color-white);
		line-height: 1.3;
		letter-spacing: -0.015em;
		margin: 0.625rem 0 0;
	}

	.members-page__empty-body {
		font-size: 0.875rem;
		font-weight: 400;
		color: var(--color-grey-400);
		line-height: 1.5;
		margin: 0.5rem 0 0;
		max-width: min(19rem, 36ch);
	}

	.members-page__empty-actions {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.5rem;
		width: 100%;
		margin-top: 1.25rem;
		max-width: 18rem;
	}

	@media (min-width: 480px) {
		.members-page__empty-actions {
			flex-direction: row;
			justify-content: center;
			flex-wrap: wrap;
		}
	}

	.members-page__empty-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 2.5rem;
		padding: 0.5rem 1rem;
		border-radius: var(--radius-lg, 0.5rem);
		font-size: 0.8125rem;
		font-weight: 600;
		font-family: var(--font-ui);
		text-decoration: none;
		cursor: pointer;
		transition:
			background-color 150ms ease,
			color 150ms ease,
			border-color 150ms ease,
			box-shadow 150ms ease;
		border: 1px solid transparent;
	}

	.members-page__empty-btn:focus-visible {
		outline: 2px solid var(--color-teal, #0fa4af);
		outline-offset: 2px;
	}

	.members-page__empty-btn--primary {
		background: var(--color-teal, #0fa4af);
		color: #fff;
		border-color: rgba(0, 0, 0, 0.08);
		box-shadow: 0 1px 0 rgba(255, 255, 255, 0.15) inset;
	}

	.members-page__empty-btn--primary:hover {
		background: var(--color-teal-600, #0d8c95);
	}

	.members-page__empty-btn--secondary {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-200);
		border-color: rgba(255, 255, 255, 0.14);
	}

	.members-page__empty-btn--secondary:hover {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.2);
	}

	.members-page__cards {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.members-page__table-wrap {
		display: none;
	}

	.member-card {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		box-shadow: 0 8px 16px -4px rgba(0, 0, 0, 0.2);
		transition: all 250ms var(--ease-out);
	}

	.member-card[data-state='banned'] {
		border-color: rgba(239, 68, 68, 0.32);
	}

	.member-card[data-state='suspended'] {
		border-color: rgba(245, 158, 11, 0.32);
	}

	.member-card:hover {
		transform: translateY(-1px);
	}

	.member-card__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.member-card__identity {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		min-width: 0;
	}

	.member-card__avatar {
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-full);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, var(--color-teal-dark), var(--color-deep-blue));
		color: var(--color-white);
		font-size: 0.75rem;
		font-weight: 700;
		flex-shrink: 0;
		text-transform: uppercase;
	}

	.member-card__name-block {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.member-card__name {
		font-weight: 600;
		color: var(--color-white);
		font-size: 0.875rem;
		line-height: 1.35;
	}

	.member-card__email {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--color-grey-400);
		line-height: 1.4;
		word-break: break-all;
	}

	.member-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.member-card__label {
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.member-card__value {
		font-size: 0.875rem;
		font-weight: 400;
		color: var(--color-grey-300);
		text-align: right;
	}

	.member-card__pill {
		font-size: 0.6875rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
	}

	.member-card__pill--active {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}

	.member-card__pill--suspended {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.member-card__pill--banned {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.member-card__profile {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-teal-light);
		text-decoration: none;
		padding: 0.4rem 0.6rem;
		border-radius: var(--radius-md);
		background: rgba(15, 164, 175, 0.08);
		align-self: flex-start;
		transition: background 150ms var(--ease-out);
	}

	.member-card__profile:hover {
		background: rgba(15, 164, 175, 0.18);
	}

	.member-card__role {
		font-size: 0.6875rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		text-transform: capitalize;
		flex-shrink: 0;
	}

	.member-card__role--admin {
		background-color: rgba(245, 158, 11, 0.12);
		color: #f59e0b;
	}

	.member-card__role--member {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}

	.member-card__actions {
		display: flex;
		gap: 0.4rem;
		align-items: center;
		justify-content: space-between;
		margin-top: 0.25rem;
		padding-top: 0.625rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.member-card__menu-trigger {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-radius: var(--radius-md);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			color 150ms var(--ease-out),
			border-color 150ms var(--ease-out);
	}

	.member-card__menu-trigger:hover,
	.member-card__menu-trigger:focus-visible,
	.member-card__menu-trigger[aria-expanded='true'] {
		background-color: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		outline: none;
	}

	.member-card__menu-trigger:focus-visible {
		border-color: rgba(15, 164, 175, 0.5);
	}

	.members-page__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1.25rem;
	}

	.members-page__page-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		min-height: 3rem;
		padding: 0 1.25rem;
		background: rgba(255, 255, 255, 0.05);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		color: var(--color-white);
		font-size: 0.8125rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}

	.members-page__page-btn:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.08);
		border-color: rgba(15, 164, 175, 0.4);
	}

	.members-page__page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.members-page__page-info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
	}

	@media (min-width: 768px) {
		.members-page__page-header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
			margin-bottom: 1.5rem;
		}

		.members-page__cta {
			align-self: flex-end;
		}

		.members-page__toolbar {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
			gap: 1rem;
			margin-bottom: 1.25rem;
			flex-wrap: wrap;
		}

		.members-page__search {
			flex: 1;
			max-width: 28rem;
		}

		.members-page__cards {
			display: none;
		}

		.members-page__table-wrap {
			display: block;
			/* `hidden` on one axis forces the other to `auto` (unwanted vertical bar). */
			overflow-x: clip;
			overflow-y: visible;
			background: rgba(19, 43, 80, 0.48);
			/* Skip backdrop-filter here: it creates a fixed-position containing block and clips tooltips. */
			border: 1px solid rgba(255, 255, 255, 0.08);
			border-radius: var(--radius-2xl);
			box-shadow:
				0 1px 0 rgba(255, 255, 255, 0.05) inset,
				0 16px 32px -8px rgba(0, 0, 0, 0.3);
		}

		.m-table {
			width: 100%;
			table-layout: fixed;
			border-collapse: collapse;
		}

		.m-table thead {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.m-table th {
			text-align: left;
			font-size: 0.6875rem;
			font-weight: 700;
			color: var(--color-grey-500);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			padding: 0.875rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.m-table__th-actions {
			text-align: right;
		}

		.m-table td {
			padding: 0.875rem 1rem;
			font-size: 0.875rem;
			font-weight: 400;
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			vertical-align: middle;
		}

		.m-table tbody tr {
			transition: background-color 150ms var(--ease-out);
		}

		.m-table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.m-table tbody tr:last-child td {
			border-bottom: none;
		}

		.m-table__identity {
			display: flex;
			align-items: center;
			gap: 0.625rem;
			min-width: 0;
		}

		.m-table__identity--btn {
			background: none;
			border: none;
			padding: 0;
			cursor: pointer;
			color: inherit;
			font: inherit;
			min-width: 0;
			max-width: 100%;
		}

		.m-table__avatar {
			width: 1.875rem;
			height: 1.875rem;
			border-radius: var(--radius-full);
			display: inline-flex;
			align-items: center;
			justify-content: center;
			background: linear-gradient(135deg, var(--color-teal-dark), var(--color-deep-blue));
			color: var(--color-white);
			font-size: 0.6875rem;
			font-weight: 700;
			flex-shrink: 0;
			text-transform: uppercase;
		}

		.m-table__name {
			font-weight: 600;
			color: var(--color-white);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
			min-width: 0;
		}

		.m-table__muted {
			color: var(--color-grey-400);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}

		.m-table__role {
			display: inline-block;
			font-size: 0.6875rem;
			font-weight: 600;
			letter-spacing: 0.04em;
			padding: 0.2rem 0.6rem;
			border-radius: var(--radius-full);
			text-transform: capitalize;
		}

		.m-table__role--admin {
			background-color: rgba(245, 158, 11, 0.12);
			color: #f59e0b;
		}

		.m-table__role--member {
			background-color: rgba(15, 164, 175, 0.12);
			color: var(--color-teal);
		}

		.m-table__pill {
			display: inline-block;
			font-size: 0.6875rem;
			font-weight: 600;
			letter-spacing: 0.04em;
			padding: 0.2rem 0.6rem;
			border-radius: var(--radius-full);
		}

		.m-table__pill--active {
			background: rgba(34, 197, 94, 0.15);
			color: #22c55e;
		}

		.m-table__pill--suspended {
			background: rgba(245, 158, 11, 0.15);
			color: #f59e0b;
		}

		.m-table__pill--banned {
			background: rgba(239, 68, 68, 0.15);
			color: #ef4444;
		}

		.m-table__actions {
			display: flex;
			justify-content: flex-end;
			gap: 0.4rem;
		}

		.m-table__menu-trigger {
			display: inline-flex;
			align-items: center;
			justify-content: center;
			width: 2rem;
			height: 2rem;
			padding: 0;
			border-radius: var(--radius-md);
			border: 1px solid transparent;
			background: transparent;
			color: var(--color-grey-300);
			cursor: pointer;
			transition:
				background-color 150ms var(--ease-out),
				color 150ms var(--ease-out),
				border-color 150ms var(--ease-out);
		}

		.m-table__menu-trigger:hover,
		.m-table__menu-trigger:focus-visible,
		.m-table__menu-trigger[aria-expanded='true'] {
			background-color: rgba(255, 255, 255, 0.08);
			color: var(--color-white);
			outline: none;
		}

		.m-table__menu-trigger:focus-visible {
			border-color: rgba(15, 164, 175, 0.5);
		}

		.members-page__pagination {
			gap: 1rem;
			margin-top: 1.5rem;
		}

		.members-page__page-btn {
			padding: 0.55rem 1rem;
		}

		/* CLS guard (tablet+, table view): real `.m-table` rows are
		   ~3.6rem tall (`.m-table td` padding 0.875rem × 2 + ~1rem
		   content). Lock the skeleton to the same physical footprint
		   so the table doesn't grow upward when the data lands. */
		.members-page__skeleton-row {
			height: 3.6rem;
			border-radius: var(--radius-md);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.members-page__skeleton-row {
			animation: none;
		}
	}
</style>
