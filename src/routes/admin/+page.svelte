<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { AdminStats } from '$lib/api/types';
	import Users from 'phosphor-svelte/lib/Users';
	import Lightning from 'phosphor-svelte/lib/Lightning';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';
	import CalendarCheck from 'phosphor-svelte/lib/CalendarCheck';

	let stats = $state<AdminStats | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			stats = await api.get<AdminStats>('/api/admin/stats');
		} catch {
			// handle
		} finally {
			loading = false;
		}
	});

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Admin Dashboard - Precision Options Signals</title>
</svelte:head>

<div class="admin-dash">
	<h1 class="admin-dash__title">Admin Dashboard</h1>

	{#if loading}
		<p class="admin-dash__loading">Loading stats...</p>
	{:else if stats}
		<!-- KPI Cards -->
		<div class="admin-dash__kpis">
			<div class="kpi">
				<div class="kpi__icon kpi__icon--blue">
					<Users size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Total Members</p>
					<p class="kpi__value">{stats.total_members.toLocaleString()}</p>
				</div>
			</div>

			<div class="kpi">
				<div class="kpi__icon kpi__icon--green">
					<Lightning size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Active Subscriptions</p>
					<p class="kpi__value">{stats.active_subscriptions.toLocaleString()}</p>
				</div>
			</div>

			<div class="kpi">
				<div class="kpi__icon kpi__icon--teal">
					<CalendarCheck size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Monthly / Annual</p>
					<p class="kpi__value">{stats.monthly_subscriptions} / {stats.annual_subscriptions}</p>
				</div>
			</div>

			<div class="kpi">
				<div class="kpi__icon kpi__icon--purple">
					<ListChecks size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Watchlists Published</p>
					<p class="kpi__value">{stats.total_watchlists}</p>
				</div>
			</div>

			<div class="kpi">
				<div class="kpi__icon kpi__icon--amber">
					<BookOpen size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Course Enrollments</p>
					<p class="kpi__value">{stats.total_enrollments}</p>
				</div>
			</div>

			<div class="kpi">
				<div class="kpi__icon kpi__icon--rose">
					<TrendUp size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Conversion Rate</p>
					<p class="kpi__value">
						{stats.total_members > 0
							? ((stats.active_subscriptions / stats.total_members) * 100).toFixed(1)
							: 0}%
					</p>
				</div>
			</div>
		</div>

		<!-- Recent Members -->
		<section class="admin-dash__section">
			<div class="admin-dash__section-header">
				<h2 class="admin-dash__section-title">Recent Members</h2>
				<a href="/admin/members" class="admin-dash__link">View all →</a>
			</div>

			{#if stats.recent_members.length === 0}
				<p class="admin-dash__empty">No members yet.</p>
			{:else}
				<!-- Mobile: Card view -->
				<div class="admin-dash__cards">
					{#each stats.recent_members as member (member.id)}
						<div class="member-card">
							<div class="member-card__row">
								<span class="member-card__label">Name</span>
								<span class="member-card__value member-card__name">{member.name}</span>
							</div>
							<div class="member-card__row">
								<span class="member-card__label">Email</span>
								<span class="member-card__value">{member.email}</span>
							</div>
							<div class="member-card__row">
								<span class="member-card__label">Role</span>
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
								<span class="member-card__label">Joined</span>
								<span class="member-card__value">{formatDate(member.created_at)}</span>
							</div>
						</div>
					{/each}
				</div>
				<!-- Tablet+: Table view -->
				<div class="admin-dash__table-wrap">
					<table class="admin-table">
						<thead>
							<tr>
								<th>Name</th>
								<th>Email</th>
								<th>Role</th>
								<th>Joined</th>
							</tr>
						</thead>
						<tbody>
							{#each stats.recent_members as member (member.id)}
								<tr>
									<td class="admin-table__name">{member.name}</td>
									<td>{member.email}</td>
									<td>
										<span
											class={[
												'admin-table__role',
												member.role === 'admin'
													? 'admin-table__role--admin'
													: 'admin-table__role--member'
											]}
										>
											{member.role}
										</span>
									</td>
									<td>{formatDate(member.created_at)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>

		<!-- Quick Actions -->
		<section class="admin-dash__section">
			<h2 class="admin-dash__section-title">Quick Actions</h2>
			<div class="admin-dash__actions">
				<a href="/admin/watchlists/new" class="admin-dash__action">
					<ListChecks size={22} weight="duotone" />
					Create New Watchlist
				</a>
				<a href="/admin/members" class="admin-dash__action">
					<Users size={22} weight="duotone" />
					Manage Members
				</a>
			</div>
		</section>
	{/if}
</div>

<style>
	/* Mobile-first base styles */
	.admin-dash__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 1.5rem;
	}

	.admin-dash__loading {
		color: var(--color-grey-400);
		text-align: center;
		padding: 2rem;
	}

	.admin-dash__kpis {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.75rem;
		margin-bottom: 2rem;
	}

	.kpi {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}

	.kpi__icon {
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-lg);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.kpi__icon--blue {
		background-color: rgba(59, 130, 246, 0.15);
		color: #3b82f6;
	}
	.kpi__icon--green {
		background-color: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}
	.kpi__icon--teal {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}
	.kpi__icon--purple {
		background-color: rgba(168, 85, 247, 0.15);
		color: #a855f7;
	}
	.kpi__icon--amber {
		background-color: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}
	.kpi__icon--rose {
		background-color: rgba(244, 63, 94, 0.15);
		color: #f43f5e;
	}

	.kpi__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-bottom: 0.1rem;
	}

	.kpi__value {
		font-size: var(--fs-base);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.admin-dash__section {
		margin-bottom: 2rem;
	}

	.admin-dash__section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
		gap: 1rem;
	}

	.admin-dash__section-title {
		font-size: var(--fs-base);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.admin-dash__link {
		font-size: var(--fs-xs);
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		text-decoration: none;
		white-space: nowrap;
	}

	.admin-dash__link:hover {
		text-decoration: underline;
	}

	.admin-dash__empty {
		color: var(--color-grey-400);
		text-align: center;
		padding: 1.5rem;
	}

	/* Mobile: Card view for members */
	.admin-dash__table-wrap {
		display: block;
	}

	.admin-table {
		display: none;
	}

	.admin-dash__cards {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.member-card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
	}

	.member-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.member-card__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.member-card__value {
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		text-align: right;
	}

	.member-card__name {
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.member-card__role {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: capitalize;
	}

	.member-card__role--admin {
		background-color: rgba(245, 158, 11, 0.12);
		color: #f59e0b;
	}

	.member-card__role--member {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}

	.admin-dash__actions {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.admin-dash__action {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: border-color 200ms var(--ease-out);
	}

	.admin-dash__action:hover {
		border-color: rgba(15, 164, 175, 0.3);
	}

	/* Tablet: 2-column KPIs, show table */
	@media (min-width: 480px) {
		.admin-dash__kpis {
			grid-template-columns: repeat(2, 1fr);
		}

		.admin-dash__actions {
			flex-direction: row;
			flex-wrap: wrap;
		}

		.admin-dash__action {
			flex: 1;
			min-width: 10rem;
		}
	}

	@media (min-width: 768px) {
		.admin-dash__title {
			font-size: var(--fs-2xl);
			margin-bottom: 2rem;
		}

		.admin-dash__kpis {
			grid-template-columns: repeat(3, 1fr);
			gap: 1rem;
			margin-bottom: 2.5rem;
		}

		.kpi {
			padding: 1.15rem;
			gap: 1rem;
		}

		.kpi__icon {
			width: 2.75rem;
			height: 2.75rem;
		}

		.kpi__value {
			font-size: var(--fs-lg);
		}

		.admin-dash__section {
			margin-bottom: 2.5rem;
		}

		.admin-dash__section-title {
			font-size: var(--fs-lg);
		}

		/* Show table, hide cards */
		.admin-dash__table-wrap {
			overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
		}

		.admin-table {
			display: table;
			width: 100%;
			border-collapse: collapse;
		}

		.admin-dash__cards {
			display: none;
		}

		.admin-table th {
			text-align: left;
			font-size: var(--fs-xs);
			font-weight: var(--w-semibold);
			color: var(--color-grey-400);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			padding: 0.75rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.admin-table td {
			padding: 0.85rem 1rem;
			font-size: var(--fs-sm);
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		}

		.admin-table__name {
			font-weight: var(--w-semibold);
			color: var(--color-white);
		}

		.admin-table__role {
			font-size: var(--fs-xs);
			font-weight: var(--w-semibold);
			padding: 0.15rem 0.55rem;
			border-radius: var(--radius-full);
			text-transform: capitalize;
		}

		.admin-table__role--admin {
			background-color: rgba(245, 158, 11, 0.12);
			color: #f59e0b;
		}

		.admin-table__role--member {
			background-color: rgba(15, 164, 175, 0.12);
			color: var(--color-teal);
		}

		.admin-dash__action {
			padding: 0.85rem 1.25rem;
		}

		.admin-dash__action:hover {
			transform: translateY(-2px);
		}
	}

	@media (min-width: 1024px) {
		.admin-dash__kpis {
			grid-template-columns: repeat(6, 1fr);
		}

		.kpi {
			flex-direction: column;
			text-align: center;
			gap: 0.5rem;
		}

		.kpi__icon {
			width: 3rem;
			height: 3rem;
		}
	}
</style>
