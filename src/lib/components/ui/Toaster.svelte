<!--
  Toaster — global toast stack.

  Mounts once in the root layout (`+layout.svelte`) and renders every entry
  from `toast` (`$lib/stores/toast.svelte.ts`). Sonner / Linear / Stripe
  Dashboard polish: top-right on desktop, full-width-top on mobile, slide-in
  from the edge, fade-out on dismiss, hover-pause auto-dismiss, keyboard
  `Esc`, variant-tinted accent border.

  A11y:
  - Region is split into two lists — polite (success/info) and assertive
    (warning/error) — so screen readers route urgency correctly.
  - Each toast is `role="status"` and `aria-atomic` so the full title +
    description re-announces as a unit.
  - The dismiss button has an accessible name; the action button (when
    provided) keeps natural button semantics.
  - SSR renders nothing; the live stack only hydrates client-side so we
    never serialise transient state into the HTML stream.
-->
<script lang="ts">
	import { browser } from '$app/environment';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import { fly, fade } from 'svelte/transition';
	import { cubicOut, cubicIn } from 'svelte/easing';
	import { MediaQuery } from 'svelte/reactivity';
	import { toast, type ToastItem, type ToastVariant } from '$lib/stores/toast.svelte';

	// `MediaQuery` (svelte/reactivity) is the canonical replacement for the
	// effect/listener pattern — `current` is a reactive boolean that flips on
	// browser viewport / setting changes and SSR-safely defaults to `false`.
	const reducedMotionMq = new MediaQuery('prefers-reduced-motion: reduce');
	const mobileMq = new MediaQuery('max-width: 767px');
	const reducedMotion = $derived(reducedMotionMq.current);
	const isMobile = $derived(mobileMq.current);

	// Newest on top: `toast.items` is oldest-first, so we reverse for
	// presentation. The split into polite vs assertive matches WAI-ARIA
	// guidance — success/info are non-urgent, warnings/errors interrupt.
	const ordered = $derived([...toast.items].slice().reverse());
	const polite = $derived(ordered.filter((t) => t.variant === 'success' || t.variant === 'info'));
	const assertive = $derived(
		ordered.filter((t) => t.variant === 'error' || t.variant === 'warning')
	);

	function variantIcon(v: ToastVariant) {
		switch (v) {
			case 'success':
				return CheckCircleIcon;
			case 'error':
				return XCircleIcon;
			case 'warning':
				return WarningIcon;
			case 'info':
			default:
				return InfoIcon;
		}
	}
</script>

{#snippet toastCard(t: ToastItem)}
	{@const Icon = variantIcon(t.variant)}
	{@const autoDismiss = (node: HTMLElement) => {
		// Per-toast auto-dismiss timer with hover-pause. The `data-paused`
		// attribute on the same element is toggled by mouseenter/leave below;
		// we sample it once per frame via rAF so a hover at any point freezes
		// the countdown without us re-listening on every ancestor.
		if (t.duration <= 0) return;
		let remaining = t.duration;
		let last = performance.now();
		let raf = 0;
		const tick = (now: number) => {
			const dt = now - last;
			last = now;
			const paused = node.dataset.paused === 'true';
			if (!paused) remaining -= dt;
			if (remaining <= 0) {
				toast.dismiss(t.id);
				return;
			}
			raf = requestAnimationFrame(tick);
		};
		raf = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(raf);
	}}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<li
		class="toast toast--{t.variant}"
		role="status"
		aria-atomic="true"
		data-toast-id={t.id}
		{@attach autoDismiss}
		in:fly={{
			y: reducedMotion ? 0 : isMobile ? -16 : 0,
			x: reducedMotion ? 0 : isMobile ? 0 : 24,
			duration: reducedMotion ? 0 : 220,
			easing: cubicOut
		}}
		out:fade={{ duration: reducedMotion ? 0 : 180, easing: cubicIn }}
		onmouseenter={(e) => {
			(e.currentTarget as HTMLElement).dataset.paused = 'true';
		}}
		onmouseleave={(e) => {
			(e.currentTarget as HTMLElement).dataset.paused = 'false';
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') {
				e.stopPropagation();
				toast.dismiss(t.id);
			}
		}}
	>
		<span class="toast__icon" aria-hidden="true">
			<Icon size={18} weight="fill" />
		</span>
		<div class="toast__body">
			<p class="toast__title">{t.title}</p>
			{#if t.description}
				<p class="toast__desc">{t.description}</p>
			{/if}
		</div>
		{#if t.action}
			<button
				type="button"
				class="toast__action"
				onclick={() => {
					t.action?.onClick();
				}}
			>
				{t.action.label}
			</button>
		{/if}
		<button
			type="button"
			class="toast__close"
			aria-label="Dismiss notification"
			onclick={() => toast.dismiss(t.id)}
		>
			<XIcon size={14} weight="bold" />
		</button>
	</li>
{/snippet}

{#if browser}
	<ol class="toaster" class:toaster--mobile={isMobile} aria-label="Notifications">
		{#each assertive as t (t.id)}{@render toastCard(t)}{/each}
		{#each polite as t (t.id)}{@render toastCard(t)}{/each}
	</ol>
{/if}

<style>
	.toaster {
		position: fixed;
		top: 1rem;
		right: 1rem;
		z-index: 100;
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		pointer-events: none;
		max-width: min(24rem, calc(100vw - 2rem));
		width: max-content;
	}
	.toaster--mobile {
		top: 0.75rem;
		right: 0.5rem;
		left: 0.5rem;
		max-width: none;
		width: auto;
	}

	.toast {
		position: relative;
		display: grid;
		grid-template-columns: auto 1fr auto auto;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem 0.75rem 1.125rem;
		min-width: 18rem;
		max-width: 24rem;
		background: var(--color-navy-mid, #132b50);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-lg, 0.5rem);
		box-shadow:
			0 12px 32px rgba(0, 0, 0, 0.3),
			0 1px 0 rgba(255, 255, 255, 0.04) inset;
		color: var(--color-white, #fff);
		font-family: var(--font-ui, system-ui);
		pointer-events: auto;
		overflow: hidden;
	}
	.toaster--mobile .toast {
		min-width: 0;
		max-width: none;
		width: 100%;
	}

	/* Variant accent: 3px left border, tinted to match the icon. */
	.toast::before {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		width: 3px;
	}
	.toast--success::before { background: #5eead4; }
	.toast--error::before { background: #fca5a5; }
	.toast--warning::before { background: #fcd34d; }
	.toast--info::before { background: #7dd3fc; }

	.toast__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	.toast--success .toast__icon { color: #5eead4; }
	.toast--error .toast__icon { color: #fca5a5; }
	.toast--warning .toast__icon { color: #fcd34d; }
	.toast--info .toast__icon { color: #7dd3fc; }

	.toast__body {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.toast__title {
		margin: 0;
		font-size: 13px;
		font-weight: var(--w-semibold, 600);
		color: var(--color-white, #fff);
		line-height: 1.35;
		word-break: break-word;
	}
	.toast__desc {
		margin: 0;
		font-size: 12px;
		font-weight: var(--w-regular, 400);
		color: var(--color-grey-400, #8b95a8);
		line-height: 1.4;
		word-break: break-word;
	}

	.toast__action {
		flex-shrink: 0;
		min-height: 1.75rem;
		padding: 0.25rem 0.625rem;
		font-size: 12px;
		font-weight: var(--w-semibold, 600);
		font-family: inherit;
		color: var(--color-white, #fff);
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.375rem;
		cursor: pointer;
		transition:
			background-color 150ms ease-out,
			border-color 150ms ease-out;
	}
	.toast__action:hover {
		background: rgba(255, 255, 255, 0.12);
		border-color: rgba(255, 255, 255, 0.2);
	}
	.toast__action:focus-visible {
		outline: 2px solid #7dd3fc;
		outline-offset: 1px;
	}

	.toast__close {
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.5rem;
		height: 1.5rem;
		padding: 0;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		color: var(--color-grey-400, #8b95a8);
		cursor: pointer;
		transition:
			background-color 150ms ease-out,
			color 150ms ease-out;
	}
	.toast__close:hover {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white, #fff);
	}
	.toast__close:focus-visible {
		outline: 2px solid #7dd3fc;
		outline-offset: 1px;
	}

	@media (prefers-reduced-motion: reduce) {
		.toast { transition: none; }
	}
</style>
