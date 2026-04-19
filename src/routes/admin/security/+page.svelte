<script lang="ts">
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import IpInfoIcon from 'phosphor-svelte/lib/GlobeIcon';
	import UserGearIcon from 'phosphor-svelte/lib/UserGearIcon';
	import IdentificationBadgeIcon from 'phosphor-svelte/lib/IdentificationBadgeIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';

	const cards = [
		{
			href: '/admin/security/ip-allowlist',
			icon: IpInfoIcon,
			title: 'IP allowlist',
			summary:
				'CIDR rules that gate the admin panel. Add, toggle, or remove entries — the middleware rejects /api/admin/* traffic from any address that does not match an active rule.'
		},
		{
			href: '/admin/security/impersonation',
			icon: IdentificationBadgeIcon,
			title: 'Impersonation',
			summary:
				'Mint short-lived tokens to act as a member for support cases. Every mint, revoke, and downstream action is audited; the impersonated user is notified by email.'
		},
		{
			href: '/admin/security/roles',
			icon: UserGearIcon,
			title: 'Role / permission matrix',
			summary:
				'Grant or revoke individual permissions per role. Mutations hot-reload the in-memory policy snapshot without a server restart.'
		},
		{
			href: '/admin/dsar',
			icon: TrashIcon,
			title: 'DSAR & right-to-erasure',
			summary:
				'Admin-initiated data subject access requests and dual-control tombstones. (Existing module — link to the legacy CONSENT-07 surface here.)'
		},
		{
			href: '/admin/audit',
			icon: EyeIcon,
			title: 'Audit log viewer',
			summary:
				'Full-text search across every privileged action recorded in admin_actions, with JSON-path probing on the metadata payload and CSV export.'
		}
	] as const;
</script>

<svelte:head>
	<title>Security · Admin</title>
</svelte:head>

<div class="security-hub">
	<header class="security-hub__header">
		<div class="security-hub__title-row">
			<ShieldCheckIcon size={28} weight="duotone" />
			<h1 class="security-hub__title">Security</h1>
		</div>
		<p class="security-hub__subtitle">
			Operator-grade controls for the admin panel: IP allowlist, impersonation, roles &
			permissions, DSAR, and the audit log.
		</p>
	</header>

	<div class="security-hub__grid">
		{#each cards as card (card.href)}
			<a class="security-card" href={card.href} data-testid={`security-card-${card.title.toLowerCase().replace(/[^a-z]+/g, '-')}`}>
				<div class="security-card__icon">
					<card.icon size={22} weight="duotone" />
				</div>
				<h2 class="security-card__title">{card.title}</h2>
				<p class="security-card__summary">{card.summary}</p>
			</a>
		{/each}
	</div>
</div>

<style>
	.security-hub {
		max-width: 960px;
	}

	.security-hub__header {
		margin-bottom: var(--space-6);
	}

	.security-hub__title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-white);
	}

	.security-hub__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
	}

	.security-hub__subtitle {
		margin-top: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		max-width: 60ch;
	}

	.security-hub__grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: var(--space-4);
	}

	.security-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-5);
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-decoration: none;
		color: inherit;
		transition: transform 200ms var(--ease-out), border-color 200ms var(--ease-out);
	}

	.security-card:hover {
		transform: translateY(-2px);
		border-color: rgba(15, 164, 175, 0.4);
	}

	.security-card__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-lg);
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.security-card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
	}

	.security-card__summary {
		font-size: var(--fs-xs);
		line-height: var(--lh-relaxed);
		color: var(--color-grey-400);
		margin: 0;
	}

	@media (min-width: 768px) {
		.security-hub__title {
			font-size: var(--fs-2xl);
		}
	}
</style>
