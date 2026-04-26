<script lang="ts">
	import { resolve } from '$app/paths';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import GlobeIcon from 'phosphor-svelte/lib/GlobeIcon';
	import UserGearIcon from 'phosphor-svelte/lib/UserGearIcon';
	import IdentificationBadgeIcon from 'phosphor-svelte/lib/IdentificationBadgeIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';

	const cards = [
		{
			href: '/admin/security/ip-allowlist',
			icon: GlobeIcon,
			title: 'IP allowlist',
			eyebrow: 'Network',
			summary:
				'CIDR rules that gate the admin panel. Add, toggle, or remove entries — the middleware rejects /api/admin/* traffic from any address that does not match an active rule.'
		},
		{
			href: '/admin/security/impersonation',
			icon: IdentificationBadgeIcon,
			title: 'Impersonation',
			eyebrow: 'Support',
			summary:
				'Mint short-lived tokens to act as a member for support cases. Every mint, revoke, and downstream action is audited; the impersonated user is notified by email.'
		},
		{
			href: '/admin/security/roles',
			icon: UserGearIcon,
			title: 'Roles & permissions',
			eyebrow: 'Access control',
			summary:
				'Grant or revoke individual permissions per role. Mutations hot-reload the in-memory policy snapshot without a server restart.'
		},
		{
			href: '/admin/dsar',
			icon: TrashIcon,
			title: 'DSAR & right-to-erasure',
			eyebrow: 'Privacy',
			summary:
				'Admin-initiated data subject access requests and dual-control tombstones. Exports compose synchronously or via worker; erasures need a second admin to approve.'
		},
		{
			href: '/admin/audit',
			icon: EyeIcon,
			title: 'Audit log viewer',
			eyebrow: 'Forensics',
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
			<div class="security-hub__copy">
				<h1 class="security-hub__title">Security</h1>
				<p class="security-hub__subtitle">
					Operator-grade controls for the admin panel: IP allowlist, impersonation, roles
					&amp; permissions, DSAR, and the audit log.
				</p>
			</div>
		</div>
	</header>

	<div class="security-hub__grid">
		{#each cards as card (card.href)}
			<a
				class="security-card"
				href={resolve(card.href)}
				data-testid={`security-card-${card.title.toLowerCase().replace(/[^a-z]+/g, '-')}`}
			>
				<div class="security-card__head">
					<div class="security-card__icon">
						<card.icon size={20} weight="duotone" />
					</div>
					<span class="security-card__eyebrow">{card.eyebrow}</span>
				</div>
				<h2 class="security-card__title">{card.title}</h2>
				<p class="security-card__summary">{card.summary}</p>
				<span class="security-card__cta">
					Open
					<ArrowRightIcon size={14} weight="bold" />
				</span>
			</a>
		{/each}
	</div>
</div>

<style>
	.security-hub {
		max-width: 80rem;
		padding: 0 0 3rem;
	}

	.security-hub__header {
		margin-bottom: 1.5rem;
	}

	.security-hub__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}

	.security-hub__copy {
		min-width: 0;
	}

	.security-hub__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.2;
	}

	.security-hub__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
	}

	.security-hub__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.85rem;
	}

	.security-card {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		text-decoration: none;
		color: inherit;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
		transition:
			transform 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}

	.security-card:hover {
		transform: translateY(-2px);
		border-color: rgba(15, 164, 175, 0.4);
	}

	.security-card__head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.security-card__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-2xl);
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.security-card__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.security-card__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
	}

	.security-card__summary {
		margin: 0;
		font-size: 0.875rem;
		line-height: 1.55;
		color: var(--color-grey-400);
	}

	.security-card__cta {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		margin-top: auto;
		padding-top: 0.5rem;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-teal);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	@media (min-width: 480px) {
		.security-hub__grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (min-width: 768px) {
		.security-card {
			padding: 1.75rem;
			border-radius: var(--radius-2xl);
		}
	}

	@media (min-width: 1024px) {
		.security-hub__grid {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}
	}
</style>
