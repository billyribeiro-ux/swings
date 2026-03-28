<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { UserResponse, PaginatedResponse } from '$lib/api/types';
	import CaretLeft from 'phosphor-svelte/lib/CaretLeft';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';
	import Trash from 'phosphor-svelte/lib/Trash';
	import ShieldCheck from 'phosphor-svelte/lib/ShieldCheck';

	let members = $state<UserResponse[]>([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	async function loadMembers() {
		loading = true;
		try {
			const res = await api.get<PaginatedResponse<UserResponse>>(
				`/api/admin/members?page=${page}&per_page=15`
			);
			members = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch {
			// handle
		} finally {
			loading = false;
		}
	}

	onMount(loadMembers);

	async function toggleRole(member: UserResponse) {
		const newRole = member.role === 'admin' ? 'member' : 'admin';
		if (!confirm(`Change ${member.name} to ${newRole}?`)) return;

		try {
			await api.put(`/api/admin/members/${member.id}/role`, { role: newRole });
			await loadMembers();
		} catch {
			alert('Failed to update role');
		}
	}

	async function deleteMember(member: UserResponse) {
		if (!confirm(`Delete ${member.name}? This cannot be undone.`)) return;

		try {
			await api.del(`/api/admin/members/${member.id}`);
			await loadMembers();
		} catch {
			alert('Failed to delete member');
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
	<title>Members - Admin - Explosive Swings</title>
</svelte:head>

<div class="members-page">
	<div class="members-page__header">
		<div>
			<h1 class="members-page__title">Members</h1>
			<p class="members-page__count">{total} total members</p>
		</div>
	</div>

	{#if loading}
		<p class="members-page__loading">Loading members...</p>
	{:else}
		<div class="members-page__table-wrap">
			<table class="m-table">
				<thead>
					<tr>
						<th>Name</th>
						<th>Email</th>
						<th>Role</th>
						<th>Joined</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each members as member (member.id)}
						<tr>
							<td class="m-table__name">{member.name}</td>
							<td>{member.email}</td>
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
							<td>{formatDate(member.created_at)}</td>
							<td>
								<div class="m-table__actions">
									<button
										onclick={() => toggleRole(member)}
										class="m-table__btn m-table__btn--role"
										title="Toggle role"
									>
										<ShieldCheck size={16} weight="bold" />
									</button>
									<button
										onclick={() => deleteMember(member)}
										class="m-table__btn m-table__btn--delete"
										title="Delete member"
									>
										<Trash size={16} weight="bold" />
									</button>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<div class="members-page__pagination">
				<button onclick={() => { page--; loadMembers(); }} disabled={page <= 1} class="members-page__page-btn">
					<CaretLeft size={16} weight="bold" /> Prev
				</button>
				<span class="members-page__page-info">Page {page} of {totalPages}</span>
				<button onclick={() => { page++; loadMembers(); }} disabled={page >= totalPages} class="members-page__page-btn">
					Next <CaretRight size={16} weight="bold" />
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	.members-page__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1.5rem;
	}

	.members-page__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.members-page__count {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin-top: 0.25rem;
	}

	.members-page__loading {
		color: var(--color-grey-400);
		text-align: center;
		padding: 3rem;
	}

	.members-page__table-wrap {
		overflow-x: auto;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}

	.m-table {
		width: 100%;
		border-collapse: collapse;
	}

	.m-table th {
		text-align: left;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.85rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.m-table td {
		padding: 0.85rem 1rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.m-table tbody tr:hover {
		background-color: rgba(255, 255, 255, 0.02);
	}

	.m-table__name {
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.m-table__role {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		padding: 0.15rem 0.55rem;
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

	.m-table__actions {
		display: flex;
		gap: 0.5rem;
	}

	.m-table__btn {
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}

	.m-table__btn--role {
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
	}

	.m-table__btn--role:hover {
		background-color: rgba(15, 164, 175, 0.25);
	}

	.m-table__btn--delete {
		background-color: rgba(239, 68, 68, 0.08);
		color: #ef4444;
	}

	.m-table__btn--delete:hover {
		background-color: rgba(239, 68, 68, 0.2);
	}

	.members-page__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 1.5rem;
	}

	.members-page__page-btn {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.5rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		cursor: pointer;
		transition: border-color 200ms var(--ease-out);
	}

	.members-page__page-btn:hover:not(:disabled) {
		border-color: var(--color-teal);
	}

	.members-page__page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.members-page__page-info {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
	}
</style>
