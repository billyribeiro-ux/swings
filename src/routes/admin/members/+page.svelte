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

	// ─── Ban / Suspend inline modal ───────────────────────────────────────────
	// Replaces window.prompt() (unstyled, blocks event loop, doesn't match dark
	// theme). Resolves via Promise so existing async callers stay unchanged.
	type ModalMode = 'ban' | 'suspend';
	let modalMember = $state<UserResponse | null>(null);
	let modalMode = $state<ModalMode>('ban');
	let modalReason = $state('');
	let modalDays = $state('');
	let modalResolve = $state<((v: { reason: string; days: string } | null) => void) | null>(null);

	function openActionModal(member: UserResponse, mode: ModalMode): Promise<{ reason: string; days: string } | null> {
		return new Promise((resolve) => {
			modalMember = member;
			modalMode = mode;
			modalReason = '';
			modalDays = '';
			modalResolve = resolve;
		});
	}

	function closeActionModal(submit: boolean) {
		if (modalResolve) {
			modalResolve(submit ? { reason: modalReason, days: modalDays } : null);
		}
		modalMember = null;
		modalResolve = null;
	}

	/**
	 * Page size — single source of truth for both the API query and the
	 * reserved layout box. The cards wrapper and the table-wrap each have a
	 * CSS `min-height` derived from `PER_PAGE × row-height`, so the box
	 * stays the same size whether the filter returns 15, 3, or 0 rows. The
	 * skeleton renders exactly `PER_PAGE` rows for the same reason — first-
	 * paint and first-data have identical box dimensions.
	 */
	const PER_PAGE = 15;

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

	/**
	 * @param silent - when true, skip the `loading = true` skeleton swap.
	 *   Used after mutations (delete / role-change / suspend) where the row
	 *   has already been removed/updated optimistically; the refetch only
	 *   re-syncs `total` / `totalPages` / pagination state. Without `silent`
	 *   every mutation would briefly flash the skeleton, which Web Vitals
	 *   counts as layout shift (CLS spike on every delete).
	 */
	async function loadMembers(opts: { silent?: boolean } = {}) {
		const gen = ++membersLoadGeneration;
		if (!opts.silent) loading = true;
		try {
			// We hand-build the query string instead of allocating a
			// `URLSearchParams` because `svelte/prefer-svelte-reactivity`
			// flags the mutable instance and a non-reactive plain string
			// is what we actually want here (the request fires once per
			// `loadMembers()`).
			const q = search.trim();
			const parts: string[] = [`page=${page}`, `per_page=${PER_PAGE}`];
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
			if (gen === membersLoadGeneration && !opts.silent) loading = false;
		}
	}

	onMount(loadMembers);

	function scheduleSearchReload() {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			page = 1;
			loadMembers({ silent: true });
		}, 280);
	}

	/** Run search immediately (Enter in the field, search form submit). */
	function commitSearchNow(e: Event) {
		e.preventDefault();
		clearTimeout(searchTimeout);
		page = 1;
		loadMembers({ silent: true });
	}

	onDestroy(() => clearTimeout(searchTimeout));

	function clearMemberFilters() {
		clearTimeout(searchTimeout);
		search = '';
		roleFilter = '';
		statusFilter = '';
		page = 1;
		loadMembers({ silent: true });
	}

	const hasMemberFilters = $derived(Boolean(search.trim() || roleFilter || statusFilter));

	/**
	 * Render the empty-state panel *inline*, inside the reserved list box,
	 * when a filter returns no rows. The wrapper has a CSS `min-height`
	 * sized for `PER_PAGE` rows, so loading → real-data → empty → real-data
	 * transitions never resize the wrapper — i.e. nothing on the page below
	 * the table moves on a filter swap.
	 */
	const showInlineEmpty = $derived(!loading && members.length === 0);

	function changeRole(r: typeof roleFilter) {
		roleFilter = r;
		page = 1;
		// Silent refetch: list is already mounted; flipping `loading=true` here
		// would unmount the table and remount the 5-row skeleton, then remount
		// the table at the new row count — two layout shifts per filter click.
		loadMembers({ silent: true });
	}

	function changeStatus(s: StatusFilter) {
		statusFilter = s;
		page = 1;
		loadMembers({ silent: true });
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
				// Silent refetch — keep the table mounted, re-sync state.
				await loadMembers({ silent: true });
			} catch (e) {
				toast.error(e instanceof ApiError ? e.message : 'Failed to lift ban');
			}
			return;
		}
		const result = await openActionModal(member, 'ban');
		if (!result) return;
		try {
			await api.post(`/api/admin/members/${member.id}/ban`, { reason: result.reason });
			toast.success('Member banned');
			await loadMembers({ silent: true });
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
				await loadMembers({ silent: true });
			} catch (e) {
				toast.error(e instanceof ApiError ? e.message : 'Failed to lift suspension');
			}
			return;
		}
		const result = await openActionModal(member, 'suspend');
		if (!result) return;
		let until: string | undefined;
		if (result.days && Number.parseInt(result.days, 10) > 0) {
			const ts = new Date(Date.now() + Number.parseInt(result.days, 10) * 86400000);
			until = ts.toISOString();
		}
		try {
			await api.post(`/api/admin/members/${member.id}/suspend`, { reason: result.reason, until });
			toast.success(
				until
					? `Suspended until ${new Date(until).toLocaleDateString()}`
					: 'Member suspended'
			);
			await loadMembers({ silent: true });
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
		// Optimistic update: drop the row from the local list immediately so
		// the table doesn't reflow back to the skeleton on refetch.
		// Without this, every delete flashes the loading skeleton for ~200ms
		// — which counts as layout shift in Web Vitals (CLS spike per click).
		const previous = members;
		const previousTotal = total;
		members = members.filter((m) => m.id !== member.id);
		total = Math.max(0, total - 1);
		try {
			await api.del(`/api/admin/members/${member.id}`);
			toast.success('Member deleted');
			// Silent refetch — re-syncs `total` / `totalPages` / pagination
			// without flipping `loading=true`, so the table stays mounted.
			await loadMembers({ silent: true });
		} catch (e) {
			// Roll back the optimistic removal so the user sees the row
			// is still there if the backend rejected the delete.
			members = previous;
			total = previousTotal;
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
			await loadMembers({ silent: true });
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
			<div class="members-page__title-row">
				<h1 class="members-page__title">Members</h1>
				<!--
					Live count badge — separated from the descriptive subtitle so the
					dynamic `total` value can change without reflowing the surrounding
					paragraph. (When the count was inline in the paragraph, the
					character-count change between `0` and the real total shifted the
					line wrap, pushing every element below the header down by one line
					— a load-time CLS spike of ~0.034 on desktop.)
				-->
				<span
					class="members-page__count"
					aria-live="polite"
					aria-label="Total members"
					title="Total members"
				>
					{total.toLocaleString()}
				</span>
			</div>
			<p class="members-page__subtitle">
				Manage roles, lifecycle, and billing profile. To add someone new, use
				<strong>Search &amp; create</strong>
				(role, temp password, optional setup email).
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

	<!--
		Reserved-space layout:

		Both the cards block (mobile) and the table-wrap (tablet+) are ALWAYS
		mounted, with a CSS `min-height` sized for `PER_PAGE` rows. The inner
		content swaps between skeleton / empty-state / real rows but the
		container box never changes size. This is what kills CLS on filter
		swaps — the page below the list cannot move because the list slot is
		fixed-height regardless of result count. The skeleton is also sized
		to PER_PAGE rows so the first-paint → first-data transition is a
		pure in-place row swap, not a height change.
	-->

	<!-- Mobile: Card view -->
	<div
		class="members-page__cards"
		data-state={loading ? 'loading' : showInlineEmpty ? 'empty' : 'ready'}
	>
		{#if loading}
			{#each Array(PER_PAGE) as _, i (i)}
				<div class="members-page__skeleton-row" aria-hidden="true"></div>
			{/each}
		{:else if showInlineEmpty}
			{@render emptyPanel()}
		{:else}
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
								<ActionMenuItem icon={UserIcon} onclick={() => viewMember(member)}>
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
				</div>
			{/each}
		{/if}
	</div>

	<!-- Tablet+: Table view (always mounted; tbody swaps content based on state) -->
	<div
		class="members-page__table-wrap"
		data-state={loading ? 'loading' : showInlineEmpty ? 'empty' : 'ready'}
	>
		{#if showInlineEmpty}
			<!--
				Empty state lives INSIDE the table-wrap. The wrapper has a CSS
				`min-height` sized for `PER_PAGE` rows so the box doesn't shrink
				when a filter returns nothing. The panel centers within that box.
			-->
			{@render emptyPanel()}
		{:else}
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
					{#if loading}
						{#each Array(PER_PAGE) as _, i (i)}
							<tr class="m-table__skeleton-row" aria-hidden="true">
								<td colspan="6"></td>
							</tr>
						{/each}
					{:else}
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
													<DotsThreeVerticalIcon
														size={18}
														weight="bold"
													/>
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
													{member.suspended_at
														? 'Lift suspension'
														: 'Suspend sign-in'}
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
					{/if}
				</tbody>
			</table>
		{/if}
	</div>

	<!--
		Pagination slot — always mounted to reserve its vertical space. When
		`totalPages <= 1` the inner controls are visibility-hidden (their box
		is preserved). Without this, going from a multi-page filter result to
		a single-page one would unmount the entire pagination row and the
		page footer would jump up by ~3rem.
	-->
	<div
		class="members-page__pagination"
		class:members-page__pagination--reserved={totalPages <= 1}
		aria-hidden={totalPages <= 1}
	>
		{#if totalPages > 1}
			<button
				onclick={() => {
					page--;
					loadMembers({ silent: true });
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
					loadMembers({ silent: true });
				}}
				disabled={page >= totalPages}
				class="members-page__page-btn"
				aria-label="Next page"
			>
				<span>Next</span>
				<CaretRightIcon size={16} weight="bold" />
			</button>
		{:else}
			<!-- Phantom controls preserve the slot height when a single page exists. -->
			<span
				class="members-page__page-btn members-page__page-btn--placeholder"
				aria-hidden="true"
			></span>
			<span
				class="members-page__page-info members-page__page-info--placeholder"
				aria-hidden="true"
			>
				Page 1 of 1
			</span>
			<span
				class="members-page__page-btn members-page__page-btn--placeholder"
				aria-hidden="true"
			></span>
		{/if}
	</div>
</div>

{#snippet emptyPanel()}
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
{/snippet}

{#if modalMember}
	<div
		class="action-modal-backdrop"
		role="presentation"
		onclick={(e) => e.target === e.currentTarget && closeActionModal(false)}
		onkeydown={(e) => e.key === 'Escape' && closeActionModal(false)}
	>
		<div
			class="action-modal"
			role="dialog"
			aria-modal="true"
			aria-labelledby="action-modal-title"
		>
			<h2 id="action-modal-title" class="action-modal__title">
				{modalMode === 'ban' ? `Ban ${modalMember.name}?` : `Suspend ${modalMember.name}?`}
			</h2>
			<p class="action-modal__body">
				{#if modalMode === 'ban'}
					Banning revokes their session and immediately cancels any active subscription.
				{:else}
					Suspended members cannot sign in. Leave duration blank for open-ended.
				{/if}
			</p>

			<div class="action-modal__fields">
				<label class="action-modal__label" for="action-modal-reason">
					Reason <span class="action-modal__optional">(optional)</span>
				</label>
				<input
					id="action-modal-reason"
					type="text"
					class="action-modal__input"
					placeholder="e.g. Terms of service violation"
					bind:value={modalReason}
				/>

				{#if modalMode === 'suspend'}
					<label class="action-modal__label" for="action-modal-days">
						Duration in days <span class="action-modal__optional">(blank = open-ended)</span>
					</label>
					<input
						id="action-modal-days"
						type="number"
						min="1"
						class="action-modal__input"
						placeholder="e.g. 7"
						bind:value={modalDays}
					/>
				{/if}
			</div>

			<div class="action-modal__footer">
				<button type="button" class="action-modal__btn action-modal__btn--cancel" onclick={() => closeActionModal(false)}>
					Cancel
				</button>
				<button
					type="button"
					class="action-modal__btn action-modal__btn--confirm"
					class:action-modal__btn--danger={modalMode === 'ban'}
					class:action-modal__btn--warning={modalMode === 'suspend'}
					onclick={() => closeActionModal(true)}
				>
					{modalMode === 'ban' ? 'Ban member' : 'Suspend member'}
				</button>
			</div>
		</div>
	</div>
{/if}

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

	.members-page__title-row {
		display: flex;
		align-items: baseline;
		gap: 0.625rem;
		margin: 0 0 0.5rem;
		flex-wrap: wrap;
	}

	.members-page__title {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.2;
		letter-spacing: -0.01em;
		margin: 0;
	}

	/*
	 * Live count badge — sits next to the H1, isolated from the descriptive
	 * subtitle paragraph. Keeping the dynamic value in a separate inline
	 * element means the `total` number changing 0 → 27 → 5 (filter swaps,
	 * delete, etc.) cannot reflow the subtitle text below.
	 */
	.members-page__count {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 2.25rem;
		padding: 0.15rem 0.6rem;
		background: rgba(15, 164, 175, 0.12);
		color: var(--color-teal-light);
		border-radius: var(--radius-full);
		font-size: 0.75rem;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
		line-height: 1.2;
	}

	.members-page__subtitle {
		font-size: 0.875rem;
		font-weight: 400;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
		hyphens: none;
		/*
		 * Subtitle text is now 100% static (no dynamic count) so it cannot
		 * change line-wrap at runtime. No `min-height` reservation needed.
		 */
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

	/*
	 * Reserved-space layout — single source of truth.
	 *
	 * The container's height is derived from `--per-page × row-h + chrome`
	 * so it never changes when the dataset size changes. Both viewports
	 * use the same constants, just different row-h values (cards are
	 * physically taller than table rows).
	 *
	 * The CLS guarantee: with these values, going from 15-row → 0-row →
	 * 15-row results doesn't resize either container, so nothing on the
	 * page below the list moves. Verified by `e2e/admin/members-filter-cls.spec.ts`.
	 */
	.members-page {
		--per-page: 15;
		--row-h: 3.6rem; /* table row, tablet+ */
		--cards-row-h: 9rem; /* card view row, mobile (≈ 5rem content + gap) */
		--cards-gap: 0.625rem;
		--table-thead-h: 3rem;
	}

	/*
	 * Mobile cards view — fixed height = PER_PAGE × cards-row-h + gaps.
	 * Skeleton rows + empty panel + real cards all fit inside this box.
	 */
	.members-page__cards {
		display: flex;
		flex-direction: column;
		gap: var(--cards-gap);
		/* Reserved height: PER_PAGE × cards-row-h + (PER_PAGE - 1) × gap. */
		min-height: calc(
			var(--per-page) * var(--cards-row-h) + (var(--per-page) - 1) * var(--cards-gap)
		);
	}

	/* Empty panel + skeleton both fill the cards container vertically when
	   they are the only child, so the parent's `min-height` is honoured. */
	.members-page__cards[data-state='empty'],
	.members-page__cards[data-state='loading'] {
		justify-content: flex-start;
	}

	.members-page__cards[data-state='empty'] :global(.members-page__empty) {
		flex: 1;
	}

	/*
	 * Skeleton row — used by BOTH viewports.
	 * - In `.members-page__cards` (mobile): wraps `<div>` rows, height = cards-row-h.
	 * - In `.m-table__skeleton-row` (tablet+): set on the `<tr>` directly, height = row-h.
	 *
	 * Because the cards skeleton matches the real card height and the
	 * skeleton count matches PER_PAGE, the loading → ready transition is
	 * a pure in-place swap with no height delta.
	 */
	.members-page__skeleton-row {
		height: var(--cards-row-h);
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

	/*
	 * Empty state — lives INSIDE the list container (cards or table-wrap)
	 * so the container's reserved height envelopes it. The panel is centered
	 * inside whatever space the container has; it does not contribute its
	 * own min-height.
	 */
	.members-page__empty {
		display: flex;
		justify-content: center;
		align-items: center;
		padding: clamp(2rem, 5vw, 3rem) 1rem;
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

	.members-page__table-wrap {
		/* Hidden by default — promoted to grid display at ≥768px. */
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
			/*
			 * Reserved-space container.
			 * - `display: grid` makes the empty-panel center inside the
			 *   reserved box (`place-items: center`).
			 * - `min-height` is locked to PER_PAGE × row-h + the thead
			 *   block. Going from a 15-row result to a 0-row empty panel
			 *   does not shrink this container, so nothing below it moves.
			 */
			display: grid;
			place-items: stretch;
			min-height: calc(var(--per-page) * var(--row-h) + var(--table-thead-h));
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

		/* When the table is showing the inline empty panel, center the panel
		   inside the reserved box. (The grid container's default would stretch.) */
		.members-page__table-wrap[data-state='empty'] {
			place-items: center;
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
			/* Lock every row (real + skeleton) to the same physical height.
			   Without this, content-driven row heights drift between filters
			   (e.g. a long email wraps to two lines) and the table reflows. */
			height: var(--row-h);
			transition: background-color 150ms var(--ease-out);
		}

		.m-table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.m-table tbody tr:last-child td {
			border-bottom: none;
		}

		/*
		 * Skeleton rows — identical box to real rows. The shimmer lives on
		 * the inner `<td>` so the row's `border-bottom` (which sits on `<td>`)
		 * is undisturbed; using a `<tr>`-level background would not paint
		 * visibly because `<tr>` doesn't establish a backdrop in most engines.
		 */
		.m-table__skeleton-row td {
			background: linear-gradient(
				90deg,
				rgba(255, 255, 255, 0.03) 0%,
				rgba(255, 255, 255, 0.06) 50%,
				rgba(255, 255, 255, 0.03) 100%
			);
			background-size: 200% 100%;
			animation: shimmer 1.6s ease-in-out infinite;
		}

		.m-table__skeleton-row:hover td {
			background-color: transparent;
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
	}

	/*
	 * Reserved-pagination — when there is only one page of results we still
	 * render a phantom row so the slot height stays constant. Without this,
	 * filtering from a "150 banned" result set (10 pages) to "0 banned"
	 * (no pagination) would unmount the row entirely and the page footer
	 * would jump up by ~3rem.
	 */
	.members-page__pagination--reserved :global(.members-page__page-btn--placeholder) {
		visibility: hidden;
	}

	.members-page__pagination--reserved :global(.members-page__page-info--placeholder) {
		visibility: hidden;
	}

	.members-page__page-btn--placeholder {
		/* Match the real button's box exactly so the row height is identical. */
		display: inline-flex;
		min-height: 3rem;
		padding: 0 1.25rem;
		border: 1px solid transparent;
		border-radius: var(--radius-xl);
		font-size: 0.8125rem;
		font-weight: 600;
	}

	@media (prefers-reduced-motion: reduce) {
		.members-page__skeleton-row,
		.m-table__skeleton-row td {
			animation: none;
		}
	}

	/* ── Ban / Suspend modal ──────────────────────────────────────────────── */
	.action-modal-backdrop {
		position: fixed;
		inset: 0;
		z-index: 9000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
	}

	.action-modal {
		width: clamp(320px, 92vw, 440px);
		padding: 1.5rem;
		background: var(--color-navy-deep);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-xl);
		box-shadow:
			0 24px 64px rgba(0, 0, 0, 0.5),
			0 1px 0 rgba(255, 255, 255, 0.06) inset;
	}

	.action-modal__title {
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0 0 0.5rem;
		line-height: 1.3;
	}

	.action-modal__body {
		font-size: 0.875rem;
		color: var(--color-grey-300);
		line-height: 1.5;
		margin: 0 0 1.25rem;
	}

	.action-modal__fields {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.action-modal__label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.action-modal__optional {
		font-weight: 400;
		text-transform: none;
		letter-spacing: 0;
		opacity: 0.7;
	}

	.action-modal__input {
		width: 100%;
		min-height: 2.75rem;
		padding: 0.55rem 0.875rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.875rem;
		font-family: var(--font-ui);
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.action-modal__input::placeholder {
		color: var(--color-grey-500);
	}

	.action-modal__input:focus-visible {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px var(--color-teal-glow);
	}

	.action-modal__footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	@media (max-width: 480px) {
		.action-modal__footer {
			flex-direction: column-reverse;
		}
	}

	.action-modal__btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 2.75rem;
		padding: 0 1rem;
		font-size: 0.875rem;
		font-weight: 600;
		font-family: var(--font-ui);
		border-radius: var(--radius-lg);
		border: 1px solid transparent;
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			color 150ms var(--ease-out);
	}

	.action-modal__btn:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	@media (max-width: 480px) {
		.action-modal__btn {
			width: 100%;
		}
	}

	.action-modal__btn--cancel {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.1);
		color: var(--color-grey-200);
	}

	.action-modal__btn--cancel:hover {
		background: rgba(255, 255, 255, 0.09);
		border-color: rgba(255, 255, 255, 0.16);
		color: var(--color-white);
	}

	.action-modal__btn--danger {
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.4);
		color: var(--status-danger-50);
	}

	.action-modal__btn--danger:hover {
		background: rgba(239, 68, 68, 0.28);
		border-color: rgba(239, 68, 68, 0.6);
	}

	.action-modal__btn--warning {
		background: rgba(245, 158, 11, 0.15);
		border-color: rgba(245, 158, 11, 0.4);
		color: var(--status-warning-50);
	}

	.action-modal__btn--warning:hover {
		background: rgba(245, 158, 11, 0.28);
		border-color: rgba(245, 158, 11, 0.6);
	}
</style>
