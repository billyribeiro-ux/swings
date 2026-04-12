<script lang="ts">
	import { Canvas, T, useThrelte } from '@threlte/core';
	import { HTML, Float, RoundedBoxGeometry } from '@threlte/extras';
	import { interactivity } from '@threlte/extras/interactivity';
	import * as THREE from 'three';
	import { onMount } from 'svelte';

	interface KpiCard {
		label: string;
		value: string;
		sublabel: string;
		icon: string;
		color: string;
	}

	interface Props {
		mrr: number;
		arr: number;
		totalRevenue: number;
		activeSubscribers: number;
	}

	let { mrr, arr, totalRevenue, activeSubscribers }: Props = $props();

	let isMobile = $state(false);
	let mouseX = $state(0);
	let mouseY = $state(0);

	function formatDollars(cents: number): string {
		if (cents >= 100_000_00) {
			return `$${(cents / 100_00).toFixed(0)}K`;
		}
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: 'USD',
			minimumFractionDigits: 0,
			maximumFractionDigits: 0
		}).format(cents / 100);
	}

	function formatNumber(n: number): string {
		if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
		return n.toLocaleString();
	}

	let cards = $derived<KpiCard[]>([
		{
			label: 'MRR',
			value: formatDollars(mrr),
			sublabel: 'Monthly Recurring Revenue',
			icon: '\u{1F4C8}',
			color: '#0fa4af'
		},
		{
			label: 'ARR',
			value: formatDollars(arr),
			sublabel: 'Annual Recurring Revenue',
			icon: '\u{1F4CA}',
			color: '#15c5d1'
		},
		{
			label: 'Total Revenue',
			value: formatDollars(totalRevenue),
			sublabel: 'All-time Revenue',
			icon: '\u{1F4B0}',
			color: '#d4a843'
		},
		{
			label: 'Subscribers',
			value: formatNumber(activeSubscribers),
			sublabel: 'Active Subscribers',
			icon: '\u{1F465}',
			color: '#22b573'
		}
	]);

	let visibleCards = $derived(isMobile ? cards.slice(0, 2) : cards);

	onMount(() => {
		const mq = window.matchMedia('(max-width: 640px)');
		isMobile = mq.matches;
		const handler = (e: MediaQueryListEvent) => {
			isMobile = e.matches;
		};
		mq.addEventListener('change', handler);
		return () => mq.removeEventListener('change', handler);
	});

	function handleMouseMove(e: MouseEvent) {
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		mouseX = ((e.clientX - rect.left) / rect.width - 0.5) * 2;
		mouseY = ((e.clientY - rect.top) / rect.height - 0.5) * 2;
	}

	function handleMouseLeave() {
		mouseX = 0;
		mouseY = 0;
	}
</script>

<div
	class="kpi3d"
	role="presentation"
	onmousemove={handleMouseMove}
	onmouseleave={handleMouseLeave}
>
	<Canvas>
		<T.PerspectiveCamera
			makeDefault
			position.x={0}
			position.y={0}
			position.z={isMobile ? 9 : 12}
			fov={45}
		/>

		<T.AmbientLight intensity={0.6} />
		<T.DirectionalLight position.x={5} position.y={8} position.z={5} intensity={0.8} />
		<T.DirectionalLight position.x={-3} position.y={-2} position.z={4} intensity={0.3} color="#0fa4af" />

		<T.Group rotation.x={mouseY * -0.05} rotation.y={mouseX * 0.08}>
			{#each visibleCards as card, i}
				{@const totalCards = visibleCards.length}
				{@const spacing = isMobile ? 3.2 : 3.6}
				{@const offsetX = (i - (totalCards - 1) / 2) * spacing}
				{@const seedVal = i * 137}

				<Float
					speed={[0.8 + i * 0.1, 1.2 + i * 0.15, 0.5]}
					floatIntensity={[0.15, 0.25, 0.1]}
					rotationIntensity={[0.02, 0.03, 0.01]}
					seed={seedVal}
				>
					<T.Group position.x={offsetX} position.y={0} position.z={0}>
						<T.Mesh>
							<RoundedBoxGeometry args={[2.6, 3.2, 0.15]} radius={0.12} smoothness={4} />
							<T.MeshPhysicalMaterial
								color="#1a3a6b"
								metalness={0.1}
								roughness={0.15}
								transmission={0.6}
								thickness={0.5}
								transparent
								opacity={0.85}
								envMapIntensity={1}
								clearcoat={0.3}
								clearcoatRoughness={0.1}
								side={THREE.DoubleSide}
							/>
						</T.Mesh>

						<!-- Glass edge highlight -->
						<T.Mesh position.z={0.08}>
							<RoundedBoxGeometry args={[2.5, 3.1, 0.01]} radius={0.1} smoothness={4} />
							<T.MeshPhysicalMaterial
								color={card.color}
								metalness={0.5}
								roughness={0.3}
								transparent
								opacity={0.08}
								side={THREE.FrontSide}
							/>
						</T.Mesh>

						<!-- Top accent line -->
						<T.Mesh position.y={1.4} position.z={0.08}>
							<T.BoxGeometry args={[2.0, 0.03, 0.01]} />
							<T.MeshBasicMaterial color={card.color} transparent opacity={0.6} />
						</T.Mesh>

						<HTML
							position.x={0}
							position.y={0}
							position.z={0.12}
							center
							distanceFactor={isMobile ? 7 : 8}
							pointerEvents="none"
						>
							<div class="kpi3d-card" style="--accent: {card.color}">
								<div class="kpi3d-card__icon">{card.icon}</div>
								<div class="kpi3d-card__label">{card.label}</div>
								<div class="kpi3d-card__value">{card.value}</div>
								<div class="kpi3d-card__sub">{card.sublabel}</div>
							</div>
						</HTML>
					</T.Group>
				</Float>
			{/each}
		</T.Group>
	</Canvas>
</div>

{#if isMobile}
	<!-- Fallback flat cards for the other 2 KPIs on mobile -->
	<div class="kpi3d-mobile-extra">
		{#each cards.slice(2) as card}
			<div class="kpi3d-flat" style="--accent: {card.color}">
				<span class="kpi3d-flat__icon">{card.icon}</span>
				<span class="kpi3d-flat__label">{card.label}</span>
				<span class="kpi3d-flat__value">{card.value}</span>
				<span class="kpi3d-flat__sub">{card.sublabel}</span>
			</div>
		{/each}
	</div>
{/if}

<style>
	.kpi3d {
		width: 100%;
		height: 340px;
		position: relative;
		border-radius: var(--radius-xl, 0.75rem);
		overflow: hidden;
		background: linear-gradient(
			135deg,
			rgba(11, 29, 58, 0.95) 0%,
			rgba(19, 43, 80, 0.9) 50%,
			rgba(11, 29, 58, 0.95) 100%
		);
		border: 1px solid rgba(15, 164, 175, 0.1);
	}

	@media (max-width: 640px) {
		.kpi3d {
			height: 280px;
		}
	}

	.kpi3d-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 4px;
		pointer-events: none;
		user-select: none;
		width: 140px;
	}

	.kpi3d-card__icon {
		font-size: 1.6rem;
		margin-bottom: 4px;
	}

	.kpi3d-card__label {
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--accent, #0fa4af);
		opacity: 0.9;
	}

	.kpi3d-card__value {
		font-size: 1.5rem;
		font-weight: 800;
		color: #ffffff;
		font-variant-numeric: tabular-nums;
		line-height: 1.2;
		text-shadow: 0 0 20px var(--accent, rgba(15, 164, 175, 0.4));
	}

	.kpi3d-card__sub {
		font-size: 0.55rem;
		color: rgba(255, 255, 255, 0.45);
		margin-top: 2px;
	}

	/* Mobile fallback flat cards */
	.kpi3d-mobile-extra {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
		margin-top: 0.75rem;
	}

	.kpi3d-flat {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: 1rem 0.75rem;
		border-radius: var(--radius-lg, 0.5rem);
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(255, 255, 255, 0.04);
		backdrop-filter: blur(8px);
	}

	.kpi3d-flat__icon {
		font-size: 1.3rem;
		margin-bottom: 4px;
	}

	.kpi3d-flat__label {
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--accent, #0fa4af);
	}

	.kpi3d-flat__value {
		font-size: 1.3rem;
		font-weight: 800;
		color: #ffffff;
		font-variant-numeric: tabular-nums;
	}

	.kpi3d-flat__sub {
		font-size: 0.55rem;
		color: rgba(255, 255, 255, 0.45);
	}
</style>
