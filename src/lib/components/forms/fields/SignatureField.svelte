<!--
  FORM-10: Signature pad. Renders a <canvas> and stores the stroke as a
  PNG data URL — the shape validate.ts + backend accept. Pointer events
  (not touch-only) cover stylus + mouse + finger with one handler.

  A11y: canvases are opaque to screen readers, so we expose a `Clear`
  button and a visually-hidden text hint describing how to sign.
-->
<script lang="ts">
	import type { FieldProps } from '../types.ts';
	import EraserIcon from 'phosphor-svelte/lib/EraserIcon';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);

	let canvas = $state<HTMLCanvasElement | null>(null);
	let drawing = $state(false);
	let lastX = 0;
	let lastY = 0;

	function ctx(): CanvasRenderingContext2D | null {
		return canvas?.getContext('2d') ?? null;
	}

	function start(e: PointerEvent) {
		if (disabled || !canvas) return;
		drawing = true;
		const rect = canvas.getBoundingClientRect();
		lastX = e.clientX - rect.left;
		lastY = e.clientY - rect.top;
		canvas.setPointerCapture(e.pointerId);
	}

	function move(e: PointerEvent) {
		if (!drawing || !canvas) return;
		const rect = canvas.getBoundingClientRect();
		const x = e.clientX - rect.left;
		const y = e.clientY - rect.top;
		const c = ctx();
		if (!c) return;
		c.lineWidth = 2;
		c.lineCap = 'round';
		c.strokeStyle = 'oklch(0.22 0.05 252)';
		c.beginPath();
		c.moveTo(lastX, lastY);
		c.lineTo(x, y);
		c.stroke();
		lastX = x;
		lastY = y;
	}

	function end(e: PointerEvent) {
		if (!drawing || !canvas) return;
		drawing = false;
		try {
			canvas.releasePointerCapture(e.pointerId);
		} catch {
			// pointer may already be released (e.g. pointer cancelled)
		}
		onChange(field.key, canvas.toDataURL('image/png'));
	}

	function clear() {
		if (!canvas) return;
		const c = ctx();
		if (!c) return;
		c.clearRect(0, 0, canvas.width, canvas.height);
		onChange(field.key, '');
	}
</script>

<div class="fm-field fm-field--signature" class:fm-field--invalid={!!error}>
	<div class="fm-field__label">
		<label for={controlId}>
			{field.label ?? field.key}
			{#if field.required}
				<span class="fm-field__required" aria-hidden="true">*</span>
				<span class="fm-field__sr">(required)</span>
			{/if}
		</label>
	</div>
	{#if field.helpText}
		<p id={helpId} class="fm-field__help">{field.helpText}</p>
	{/if}
	<p class="fm-field__sr">Draw your signature with a mouse, finger, or stylus inside the box.</p>
	<canvas
		bind:this={canvas}
		id={controlId}
		class="fm-signature"
		width="480"
		height="160"
		aria-describedby={describedBy}
		aria-invalid={!!error}
		aria-label={field.label ?? 'Signature'}
		onpointerdown={start}
		onpointermove={move}
		onpointerup={end}
		onpointercancel={end}
		tabindex="0"
	></canvas>
	<div class="fm-signature__actions">
		<button type="button" class="fm-btn fm-btn--ghost" onclick={clear} {disabled}>
			<EraserIcon size={18} />
			<span>Clear</span>
		</button>
		{#if current.length > 0}
			<span class="fm-signature__ok">Signature captured</span>
		{/if}
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>
