<script lang="ts">
	import { onMount } from 'svelte';
	import type { Popup, PopupElement } from '$lib/api/types';
	import X from 'phosphor-svelte/lib/X';

	interface Props {
		popup: Popup;
		preview?: boolean;
		onclose?: () => void;
		onsubmit?: (formData: Record<string, unknown>) => void;
	}

	let { popup, preview: _preview = false, onclose, onsubmit }: Props = $props();

	let formValues = $state<Record<string, unknown>>({});
	let submitted = $state(false);
	let visible = $state(false);

	const style = $derived(popup.style_json);
	const elements = $derived(popup.content_json?.elements ?? []);
	const isFormPopup = $derived(elements.some((el: PopupElement) => ['input', 'email', 'textarea', 'select', 'checkbox', 'radio'].includes(el.type)));

	onMount(() => {
		requestAnimationFrame(() => {
			visible = true;
		});
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && (popup.popup_type === 'modal' || popup.popup_type === 'fullscreen')) {
			onclose?.();
		}
	}

	function handleFormSubmit(e: Event) {
		e.preventDefault();
		submitted = true;
		onsubmit?.(formValues);
	}

	function updateFormValue(key: string, value: unknown) {
		formValues = { ...formValues, [key]: value };
	}

	function getAnimationClass(animation: string): string {
		switch (animation) {
			case 'fade': return 'popup-anim--fade';
			case 'slide_up': return 'popup-anim--slide-up';
			case 'slide_down': return 'popup-anim--slide-down';
			case 'slide_left': return 'popup-anim--slide-left';
			case 'slide_right': return 'popup-anim--slide-right';
			case 'scale': return 'popup-anim--scale';
			default: return '';
		}
	}

	function getPositionClass(popupType: string): string {
		switch (popupType) {
			case 'modal': return 'popup-pos--modal';
			case 'slide_in': return 'popup-pos--slide-in';
			case 'banner': return 'popup-pos--banner';
			case 'fullscreen': return 'popup-pos--fullscreen';
			case 'floating_bar': return 'popup-pos--floating-bar';
			default: return 'popup-pos--modal';
		}
	}

</script>

<svelte:window onkeydown={handleKeydown} />

<div
	class="popup-wrapper {getPositionClass(popup.popup_type)}"
	class:popup-wrapper--visible={visible}
	class:popup-banner--bottom={popup.popup_type === 'banner' && popup.trigger_config?.position === 'bottom'}
	class:popup-banner--top={popup.popup_type === 'banner' && popup.trigger_config?.position !== 'bottom'}
	class:popup-floating--bottom={popup.popup_type === 'floating_bar' && popup.trigger_config?.position === 'bottom'}
	class:popup-floating--top={popup.popup_type === 'floating_bar' && popup.trigger_config?.position !== 'bottom'}
>
	{#if style.backdrop && (popup.popup_type === 'modal' || popup.popup_type === 'fullscreen')}
		<button
			class="popup-backdrop"
			style="background: {style.backdropColor || 'rgba(0,0,0,0.6)'}"
			onclick={() => onclose?.()}
			aria-label="Close popup"
			tabindex="-1"
		></button>
	{/if}

	<div
		class="popup-container {getAnimationClass(style.animation)}"
		class:popup-container--visible={visible}
		style="
			background: {style.background || '#132b50'};
			color: {style.textColor || '#ffffff'};
			border-radius: {style.borderRadius || '12px'};
			max-width: {style.maxWidth || '480px'};
			{style.padding ? `padding: ${style.padding};` : ''}
			{style.shadow ? `box-shadow: ${style.shadow};` : ''}
		"
		role={popup.popup_type === 'modal' || popup.popup_type === 'fullscreen' ? 'dialog' : 'complementary'}
		aria-label={popup.name}
	>
		<button
			class="popup-close"
			onclick={() => onclose?.()}
			aria-label="Close popup"
			style="color: {style.textColor || '#ffffff'}"
		>
			<X size={20} weight="bold" />
		</button>

		{#if submitted && popup.success_message}
			<div class="popup-success">
				<div class="popup-success__icon">&#10003;</div>
				<p class="popup-success__text" style="color: {style.textColor || '#ffffff'}">{popup.success_message}</p>
			</div>
		{:else}
			<form onsubmit={handleFormSubmit} class="popup-form">
				{#each elements as element (element.id)}
					{@const props = element.props}
					{@const elStyle = element.style || {}}

					{#if element.type === 'heading'}
						<h2
							class="popup-el popup-el--heading"
							style="
								color: {elStyle.color || style.textColor || '#ffffff'};
								{elStyle.fontSize ? `font-size: ${elStyle.fontSize};` : ''}
								{elStyle.textAlign ? `text-align: ${elStyle.textAlign};` : ''}
								{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}
							"
						>
							{props.text || ''}
						</h2>

					{:else if element.type === 'text'}
						<p
							class="popup-el popup-el--text"
							style="
								color: {elStyle.color || style.textColor || 'rgba(255,255,255,0.7)'};
								{elStyle.fontSize ? `font-size: ${elStyle.fontSize};` : ''}
								{elStyle.textAlign ? `text-align: ${elStyle.textAlign};` : ''}
								{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}
							"
						>
							{props.text || ''}
						</p>

					{:else if element.type === 'image'}
						<div
							class="popup-el popup-el--image"
							style="{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}"
						>
							<img
								src={String(props.src || '')}
								alt={String(props.alt || '')}
								style="
									width: {elStyle.width || '100%'};
									{elStyle.height ? `height: ${elStyle.height};` : ''}
									{elStyle.borderRadius ? `border-radius: ${elStyle.borderRadius};` : ''}
									object-fit: cover;
								"
							/>
						</div>

					{:else if element.type === 'email' || element.type === 'input'}
						<div
							class="popup-el popup-el--field"
							style="{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}"
						>
							{#if props.label}
								<label class="popup-field__label" for="popup-field-{element.id}" style="color: {style.textColor || '#ffffff'}">
									{props.label}
									{#if props.required}<span class="popup-field__required">*</span>{/if}
								</label>
							{/if}
							<input
								id="popup-field-{element.id}"
								type={element.type === 'email' ? 'email' : String(props.inputType || 'text')}
								placeholder={String(props.placeholder || '')}
								required={!!props.required}
								class="popup-field__input"
								style="--accent: {style.accentColor || '#0fa4af'}"
								oninput={(e) => updateFormValue(String(props.name || element.id), (e.target as HTMLInputElement).value)}
							/>
						</div>

					{:else if element.type === 'textarea'}
						<div
							class="popup-el popup-el--field"
							style="{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}"
						>
							{#if props.label}
								<label class="popup-field__label" for="popup-field-{element.id}" style="color: {style.textColor || '#ffffff'}">
									{props.label}
									{#if props.required}<span class="popup-field__required">*</span>{/if}
								</label>
							{/if}
							<textarea
								id="popup-field-{element.id}"
								placeholder={String(props.placeholder || '')}
								required={!!props.required}
								rows={Number(props.rows) || 3}
								class="popup-field__textarea"
								style="--accent: {style.accentColor || '#0fa4af'}"
								oninput={(e) => updateFormValue(String(props.name || element.id), (e.target as HTMLTextAreaElement).value)}
							></textarea>
						</div>

					{:else if element.type === 'select'}
						<div
							class="popup-el popup-el--field"
							style="{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}"
						>
							{#if props.label}
								<label class="popup-field__label" for="popup-field-{element.id}" style="color: {style.textColor || '#ffffff'}">
									{props.label}
									{#if props.required}<span class="popup-field__required">*</span>{/if}
								</label>
							{/if}
							<select
								id="popup-field-{element.id}"
								required={!!props.required}
								class="popup-field__select"
								style="--accent: {style.accentColor || '#0fa4af'}"
								onchange={(e) => updateFormValue(String(props.name || element.id), (e.target as HTMLSelectElement).value)}
							>
								{#if props.placeholder}
									<option value="" disabled selected>{props.placeholder}</option>
								{/if}
								{#each (props.options as Array<{label: string; value: string}>) || [] as opt (opt.value)}
									<option value={opt.value}>{opt.label}</option>
								{/each}
							</select>
						</div>

					{:else if element.type === 'checkbox'}
						<fieldset
							class="popup-el popup-el--fieldset"
							style="{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}"
						>
							{#if props.label}
								<legend class="popup-field__label" style="color: {style.textColor || '#ffffff'}">
									{props.label}
								</legend>
							{/if}
							<div class="popup-field__checks">
								{#each (props.options as Array<{label: string; value: string}>) || [] as opt (opt.value)}
									<label class="popup-field__check-label" style="color: {style.textColor || '#ffffff'}">
										<input
											type="checkbox"
											value={opt.value}
											class="popup-field__check"
											style="--accent: {style.accentColor || '#0fa4af'}"
											onchange={() => {
												const current = (formValues[String(props.name || element.id)] as string[]) || [];
												const idx = current.indexOf(opt.value);
												if (idx >= 0) {
													updateFormValue(String(props.name || element.id), current.filter((_: string, i: number) => i !== idx));
												} else {
													updateFormValue(String(props.name || element.id), [...current, opt.value]);
												}
											}}
										/>
										<span>{opt.label}</span>
									</label>
								{/each}
							</div>
						</fieldset>

					{:else if element.type === 'radio'}
						<fieldset
							class="popup-el popup-el--fieldset"
							style="{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}"
						>
							{#if props.label}
								<legend class="popup-field__label" style="color: {style.textColor || '#ffffff'}">
									{props.label}
								</legend>
							{/if}
							<div class="popup-field__checks">
								{#each (props.options as Array<{label: string; value: string}>) || [] as opt (opt.value)}
									<label class="popup-field__check-label" style="color: {style.textColor || '#ffffff'}">
										<input
											type="radio"
											name="popup-radio-{element.id}"
											value={opt.value}
											class="popup-field__check"
											style="--accent: {style.accentColor || '#0fa4af'}"
											onchange={() => updateFormValue(String(props.name || element.id), opt.value)}
										/>
										<span>{opt.label}</span>
									</label>
								{/each}
							</div>
						</fieldset>

					{:else if element.type === 'button'}
						<div
							class="popup-el popup-el--button-wrap"
							style="
								{elStyle.textAlign ? `text-align: ${elStyle.textAlign};` : 'text-align: center;'}
								{elStyle.marginTop ? `margin-top: ${elStyle.marginTop};` : ''}
								{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}
							"
						>
							<button
								type={isFormPopup ? 'submit' : 'button'}
								class="popup-btn"
								style="
									background: {elStyle.background || style.accentColor || '#0fa4af'};
									color: {elStyle.color || '#ffffff'};
									{elStyle.borderRadius ? `border-radius: ${elStyle.borderRadius};` : ''}
									{elStyle.fontSize ? `font-size: ${elStyle.fontSize};` : ''}
									{elStyle.padding ? `padding: ${elStyle.padding};` : ''}
									{elStyle.width ? `width: ${elStyle.width};` : ''}
								"
								onclick={() => {
									if (!isFormPopup) {
										onsubmit?.({});
									}
								}}
							>
								{props.text || 'Submit'}
							</button>
						</div>

					{:else if element.type === 'divider'}
						<hr
							class="popup-el popup-el--divider"
							style="
								border-color: {elStyle.color || 'rgba(255,255,255,0.1)'};
								{elStyle.marginTop ? `margin-top: ${elStyle.marginTop};` : ''}
								{elStyle.marginBottom ? `margin-bottom: ${elStyle.marginBottom};` : ''}
							"
						/>

					{:else if element.type === 'spacer'}
						<div
							class="popup-el popup-el--spacer"
							style="height: {elStyle.height || props.height || '16px'}"
						></div>
					{/if}
				{/each}
			</form>
		{/if}
	</div>
</div>

<style>
	/* Backdrop */
	.popup-backdrop {
		position: fixed;
		inset: 0;
		z-index: 9998;
		border: none;
		padding: 0;
		margin: 0;
		width: 100%;
		height: 100%;
		cursor: default;
		animation: popup-backdrop-in 300ms ease-out forwards;
	}

	@keyframes popup-backdrop-in {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	/* Wrapper positioning */
	.popup-wrapper {
		position: fixed;
		z-index: 9999;
		pointer-events: none;
	}

	.popup-wrapper--visible {
		pointer-events: auto;
	}

	.popup-pos--modal {
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-4);
	}

	.popup-pos--fullscreen {
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.popup-pos--slide-in {
		bottom: var(--space-6);
		right: var(--space-6);
	}

	.popup-pos--banner {
		left: 0;
		right: 0;
	}

	.popup-banner--top {
		top: 0;
	}

	.popup-banner--bottom {
		bottom: 0;
	}

	.popup-pos--floating-bar {
		left: 0;
		right: 0;
	}

	.popup-floating--top {
		top: 0;
	}

	.popup-floating--bottom {
		bottom: 0;
	}

	/* Container */
	.popup-container {
		position: relative;
		z-index: 9999;
		padding: var(--space-8);
		box-shadow: var(--shadow-2xl);
		overflow-y: auto;
		max-height: 90vh;
		pointer-events: auto;
		width: 100%;
	}

	.popup-pos--fullscreen .popup-container {
		width: 100%;
		height: 100%;
		max-height: 100vh;
		border-radius: 0 !important;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
	}

	.popup-pos--banner .popup-container,
	.popup-pos--floating-bar .popup-container {
		max-width: 100% !important;
		border-radius: 0 !important;
	}

	.popup-pos--slide-in .popup-container {
		width: auto;
		min-width: 320px;
	}

	/* Animations */
	.popup-anim--fade {
		opacity: 0;
		transition: opacity 300ms ease-out;
	}

	.popup-anim--fade.popup-container--visible {
		opacity: 1;
	}

	.popup-anim--slide-up {
		opacity: 0;
		transform: translateY(24px);
		transition: opacity 300ms ease-out, transform 300ms ease-out;
	}

	.popup-anim--slide-up.popup-container--visible {
		opacity: 1;
		transform: translateY(0);
	}

	.popup-anim--slide-down {
		opacity: 0;
		transform: translateY(-24px);
		transition: opacity 300ms ease-out, transform 300ms ease-out;
	}

	.popup-anim--slide-down.popup-container--visible {
		opacity: 1;
		transform: translateY(0);
	}

	.popup-anim--slide-left {
		opacity: 0;
		transform: translateX(24px);
		transition: opacity 300ms ease-out, transform 300ms ease-out;
	}

	.popup-anim--slide-left.popup-container--visible {
		opacity: 1;
		transform: translateX(0);
	}

	.popup-anim--slide-right {
		opacity: 0;
		transform: translateX(-24px);
		transition: opacity 300ms ease-out, transform 300ms ease-out;
	}

	.popup-anim--slide-right.popup-container--visible {
		opacity: 1;
		transform: translateX(0);
	}

	.popup-anim--scale {
		opacity: 0;
		transform: scale(0.9);
		transition: opacity 300ms ease-out, transform 300ms ease-out;
	}

	.popup-anim--scale.popup-container--visible {
		opacity: 1;
		transform: scale(1);
	}

	/* Close button */
	.popup-close {
		position: absolute;
		top: var(--space-3);
		right: var(--space-3);
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.1);
		cursor: pointer;
		transition: background 200ms ease-out;
		z-index: 1;
	}

	.popup-close:hover {
		background: rgba(255, 255, 255, 0.2);
	}

	/* Form */
	.popup-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	/* Elements */
	.popup-el--heading {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		line-height: var(--lh-snug);
	}

	.popup-el--text {
		margin: 0;
		font-size: var(--fs-sm);
		line-height: var(--lh-relaxed);
	}

	.popup-el--image {
		overflow: hidden;
	}

	.popup-el--image img {
		display: block;
		max-width: 100%;
		height: auto;
	}

	/* Fields */
	.popup-el--field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}

	.popup-el--fieldset {
		border: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.popup-field__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
	}

	.popup-field__required {
		color: var(--color-red);
		margin-left: 2px;
	}

	.popup-field__input,
	.popup-field__textarea,
	.popup-field__select {
		width: 100%;
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: var(--radius-lg);
		color: inherit;
		font-size: var(--fs-sm);
		font-family: var(--font-ui);
		transition: border-color 200ms ease-out;
	}

	.popup-field__input::placeholder,
	.popup-field__textarea::placeholder {
		color: rgba(255, 255, 255, 0.4);
	}

	.popup-field__input:focus,
	.popup-field__textarea:focus,
	.popup-field__select:focus {
		outline: none;
		border-color: var(--accent, var(--color-teal));
	}

	.popup-field__textarea {
		resize: vertical;
		min-height: 60px;
	}

	.popup-field__select {
		cursor: pointer;
		appearance: none;
		background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='white' viewBox='0 0 256 256'%3E%3Cpath d='M128,184a8,8,0,0,1-5.66-2.34l-80-80A8,8,0,0,1,53.66,90.34L128,164.69l74.34-74.35a8,8,0,0,1,11.32,11.32l-80,80A8,8,0,0,1,128,184Z'/%3E%3C/svg%3E");
		background-repeat: no-repeat;
		background-position: right 12px center;
		padding-right: var(--space-8);
	}

	.popup-field__select option {
		background: var(--color-navy-mid);
		color: var(--color-white);
	}

	.popup-field__checks {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.popup-field__check-label {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		cursor: pointer;
	}

	.popup-field__check {
		width: 16px;
		height: 16px;
		accent-color: var(--accent, var(--color-teal));
		cursor: pointer;
		flex-shrink: 0;
	}

	/* Button */
	.popup-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-3) var(--space-6);
		border: none;
		border-radius: var(--radius-xl);
		font-family: var(--font-ui);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: opacity 200ms ease-out, transform 200ms ease-out;
	}

	.popup-btn:hover {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	/* Divider */
	.popup-el--divider {
		border: none;
		border-top: 1px solid rgba(255, 255, 255, 0.1);
		margin: var(--space-2) 0;
	}

	/* Spacer */
	.popup-el--spacer {
		flex-shrink: 0;
	}

	/* Success */
	.popup-success {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-4);
		padding: var(--space-8) var(--space-4);
		text-align: center;
	}

	.popup-success__icon {
		width: 48px;
		height: 48px;
		border-radius: var(--radius-full);
		background: rgba(34, 181, 115, 0.2);
		color: var(--color-green);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
	}

	.popup-success__text {
		margin: 0;
		font-size: var(--fs-md);
		line-height: var(--lh-relaxed);
	}

	/* Mobile adjustments */
	@media (max-width: 480px) {
		.popup-pos--slide-in {
			bottom: var(--space-3);
			right: var(--space-3);
			left: var(--space-3);
		}

		.popup-pos--slide-in .popup-container {
			min-width: unset;
			width: 100%;
		}

		.popup-container {
			padding: var(--space-6);
		}

		.popup-el--heading {
			font-size: var(--fs-lg);
		}
	}
</style>
