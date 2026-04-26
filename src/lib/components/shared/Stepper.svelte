<!--
  Stepper — horizontal or vertical multi-step indicator.

  A11y:
  - Wrapped in `<ol>` with `role="list"` (some UA strip the implicit role when
    list-style is none, so we re-assert).
  - Active step uses `aria-current="step"`.
  - Non-interactive steps render as `<div>`; interactive steps render as
    focusable `<button>` elements that fire `onselect(id)`.
  - Arrow Left/Right (horizontal) or Up/Down (vertical) move focus between
    adjacent step buttons (roving tabindex).
  - Completed steps are visually distinguished AND announced via visually-hidden
    suffix "Completed" so AT users know the state.
-->
<script lang="ts" module>
	export type { StepperProps, StepperStep } from './Stepper.types';
</script>

<script lang="ts">
	import CheckIcon from 'phosphor-svelte/lib/CheckIcon';
	import type { StepperProps as Props } from './Stepper.types';

	const {
		steps,
		current,
		completed = [],
		orientation = 'horizontal',
		interactive = false,
		onselect,
		'aria-label': ariaLabel = 'Progress'
	}: Props = $props();

	const completedSet = $derived(new Set(completed));
	const currentIndex = $derived(steps.findIndex((s) => s.id === current));

	function stepStatus(id: string, index: number): 'complete' | 'current' | 'upcoming' {
		if (completedSet.has(id)) return 'complete';
		if (id === current) return 'current';
		if (currentIndex >= 0 && index < currentIndex) return 'complete';
		return 'upcoming';
	}

	function handleKeydown(e: KeyboardEvent, index: number) {
		if (!interactive) return;
		const nextKeys = orientation === 'horizontal' ? ['ArrowRight'] : ['ArrowDown'];
		const prevKeys = orientation === 'horizontal' ? ['ArrowLeft'] : ['ArrowUp'];
		let target: number | null = null;
		if (nextKeys.includes(e.key)) target = Math.min(steps.length - 1, index + 1);
		else if (prevKeys.includes(e.key)) target = Math.max(0, index - 1);
		else if (e.key === 'Home') target = 0;
		else if (e.key === 'End') target = steps.length - 1;
		if (target === null) return;
		e.preventDefault();
		const root = (e.currentTarget as HTMLElement).closest('ol');
		const el = root?.querySelectorAll<HTMLButtonElement>('button[data-step]')[target];
		el?.focus();
	}
</script>

<nav aria-label={ariaLabel}>
	<ol class="stepper" data-orientation={orientation} role="list">
		{#each steps as step, i (step.id)}
			{@const status = stepStatus(step.id, i)}
			<li
				class="step"
				data-status={status}
				aria-current={status === 'current' ? 'step' : undefined}
			>
				{#if interactive}
					<button
						type="button"
						class="step-inner"
						data-step={step.id}
						tabindex={status === 'current' ? 0 : -1}
						onclick={() => onselect?.(step.id)}
						onkeydown={(e) => handleKeydown(e, i)}
					>
						<span class="indicator" aria-hidden="true">
							{#if status === 'complete'}
								<CheckIcon size="0.875rem" weight="bold" />
							{:else}
								{i + 1}
							{/if}
						</span>
						<span class="copy">
							<span class="label">{step.label}</span>
							{#if step.description}<span class="description">{step.description}</span
								>{/if}
						</span>
						{#if status === 'complete'}<span class="visually-hidden">
								Completed</span
							>{/if}
					</button>
				{:else}
					<div class="step-inner">
						<span class="indicator" aria-hidden="true">
							{#if status === 'complete'}
								<CheckIcon size="0.875rem" weight="bold" />
							{:else}
								{i + 1}
							{/if}
						</span>
						<span class="copy">
							<span class="label">{step.label}</span>
							{#if step.description}<span class="description">{step.description}</span
								>{/if}
						</span>
						{#if status === 'complete'}<span class="visually-hidden">
								Completed</span
							>{/if}
					</div>
				{/if}
			</li>
		{/each}
	</ol>
</nav>

<style>
	.stepper {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		gap: var(--space-2);
	}
	.stepper[data-orientation='vertical'] {
		flex-direction: column;
		gap: var(--space-3);
	}
	.step {
		flex: 1 1 0;
		min-inline-size: 0;
	}
	.stepper[data-orientation='vertical'] .step {
		flex: 0 0 auto;
	}
	.step-inner {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		inline-size: 100%;
		background: transparent;
		border: 0;
		border-radius: var(--radius-lg);
		padding: var(--space-2);
		text-align: start;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	div.step-inner {
		cursor: default;
	}
	.indicator {
		flex: 0 0 auto;
		inline-size: 1.75rem;
		block-size: 1.75rem;
		border-radius: var(--radius-full);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		font-variant-numeric: tabular-nums;
		background-color: var(--surface-bg-muted);
		color: var(--surface-fg-muted);
		border: 1px solid var(--surface-border-subtle);
	}
	.step[data-status='current'] .indicator {
		background-color: var(--brand-teal-500);
		color: var(--neutral-0);
		border-color: var(--brand-teal-500);
	}
	.step[data-status='complete'] .indicator {
		background-color: var(--status-success-500);
		color: var(--neutral-0);
		border-color: var(--status-success-500);
	}
	.copy {
		display: flex;
		flex-direction: column;
		min-inline-size: 0;
	}
	.label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		line-height: var(--lh-snug);
	}
	.description {
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
		line-height: var(--lh-normal);
	}
	.step[data-status='current'] .label {
		color: var(--brand-teal-700);
	}
	.visually-hidden {
		position: absolute;
		inline-size: 1px;
		block-size: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
		border: 0;
	}
	@media (prefers-reduced-motion: reduce) {
		.step-inner {
			transition: none;
		}
	}
</style>
