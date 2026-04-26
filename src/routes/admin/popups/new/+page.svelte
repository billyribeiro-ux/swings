<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, ApiError } from '$lib/api/client';
	import type {
		PopupType,
		PopupTrigger,
		PopupFrequency,
		PopupElement,
		PopupStyle,
		PopupTargetingRules,
		CreatePopupPayload
	} from '$lib/api/types';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import TextHIcon from 'phosphor-svelte/lib/TextHIcon';
	import TextAaIcon from 'phosphor-svelte/lib/TextAaIcon';
	import EnvelopeSimpleIcon from 'phosphor-svelte/lib/EnvelopeSimpleIcon';
	import TextboxIcon from 'phosphor-svelte/lib/TextboxIcon';
	import CursorClickIcon from 'phosphor-svelte/lib/CursorClickIcon';
	import LineSegmentIcon from 'phosphor-svelte/lib/LineSegmentIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';

	let name = $state('');
	let popupType = $state<PopupType>('modal');
	let triggerType = $state<PopupTrigger>('on_load');
	let triggerDelayMs = $state(0);
	let triggerScrollPct = $state(50);
	let pages = $state<string[]>(['*']);
	let newPage = $state('');
	let devices = $state<PopupTargetingRules['devices']>(['desktop', 'mobile', 'tablet']);
	let userStatus = $state<PopupTargetingRules['userStatus']>(['all']);
	let bg = $state('#132b50');
	let textColor = $state('#ffffff');
	let accentColor = $state('#0fa4af');
	let borderRadius = $state('12px');
	let animation = $state<PopupStyle['animation']>('fade');
	let maxWidth = $state('480px');
	let frequency = $state<PopupFrequency>('once_per_session');
	let startsAt = $state('');
	let expiresAt = $state('');
	let elements = $state<PopupElement[]>([]);
	let editingId = $state<string | null>(null);
	let activeTab = $state<'trigger' | 'targeting' | 'style' | 'display'>('trigger');
	let saving = $state(false);
	let error = $state('');

	const popupTypes: { value: PopupType; label: string }[] = [
		{ value: 'modal', label: 'Modal' },
		{ value: 'slide_in', label: 'Slide In' },
		{ value: 'banner', label: 'Banner' },
		{ value: 'fullscreen', label: 'Fullscreen' },
		{ value: 'floating_bar', label: 'Floating Bar' }
	];
	const triggerTypes: { value: PopupTrigger; label: string }[] = [
		{ value: 'on_load', label: 'On Load' },
		{ value: 'exit_intent', label: 'Exit Intent' },
		{ value: 'scroll_percentage', label: 'Scroll %' },
		{ value: 'time_delay', label: 'Time Delay' },
		{ value: 'click', label: 'Click' },
		{ value: 'inactivity', label: 'Inactivity' }
	];
	const animations: PopupStyle['animation'][] = [
		'fade',
		'slide_up',
		'slide_down',
		'slide_left',
		'slide_right',
		'scale',
		'none'
	];
	const frequencies: { value: PopupFrequency; label: string }[] = [
		{ value: 'every_time', label: 'Every Time' },
		{ value: 'once_per_session', label: 'Once Per Session' },
		{ value: 'once_ever', label: 'Once Ever' },
		{ value: 'custom', label: 'Custom' }
	];

	function uid(): string {
		return Math.random().toString(36).slice(2, 10);
	}

	function addElement(type: PopupElement['type']) {
		const defaults: Record<string, Record<string, unknown>> = {
			heading: { text: 'Heading', level: 'h2' },
			text: { text: 'Your message here.' },
			email: { placeholder: 'Enter your email', label: 'Email', required: true },
			input: { placeholder: 'Enter text', label: 'Text', required: false },
			button: { text: 'Submit', action: 'submit' },
			divider: {}
		};
		const el: PopupElement = { id: uid(), type, props: defaults[type] ?? {} };
		elements = [...elements, el];
		editingId = el.id;
	}

	function removeElement(id: string) {
		elements = elements.filter((e) => e.id !== id);
		if (editingId === id) editingId = null;
	}

	function moveElement(id: string, dir: -1 | 1) {
		const idx = elements.findIndex((e) => e.id === id);
		if (idx < 0) return;
		const target = idx + dir;
		if (target < 0 || target >= elements.length) return;
		const copy = [...elements];
		[copy[idx], copy[target]] = [copy[target], copy[idx]];
		elements = copy;
	}

	function updateProp(id: string, key: string, value: unknown) {
		elements = elements.map((e) =>
			e.id === id ? { ...e, props: { ...e.props, [key]: value } } : e
		);
	}

	function addPage() {
		const v = newPage.trim();
		if (v && !pages.includes(v)) pages = [...pages, v];
		newPage = '';
	}

	function removePage(p: string) {
		pages = pages.filter((x) => x !== p);
	}

	function toggleDevice(d: 'desktop' | 'mobile' | 'tablet') {
		if (devices.includes(d)) devices = devices.filter((x) => x !== d);
		else devices = [...devices, d];
	}

	function toggleUserStatus(s: PopupTargetingRules['userStatus'][number]) {
		if (userStatus.includes(s)) userStatus = userStatus.filter((x) => x !== s);
		else userStatus = [...userStatus, s];
	}

	function previewText(el: PopupElement): string {
		if (el.type === 'divider') return '---';
		return String(el.props.text || el.props.label || el.props.placeholder || el.type);
	}

	function formatType(t: string): string {
		return t.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
	}

	async function save() {
		if (!name.trim()) {
			error = 'Name is required';
			return;
		}
		saving = true;
		error = '';
		const triggerConfig: Record<string, unknown> = {};
		if (triggerType === 'time_delay') triggerConfig.delay_ms = triggerDelayMs;
		if (triggerType === 'scroll_percentage') triggerConfig.scroll_pct = triggerScrollPct;

		const payload: CreatePopupPayload = {
			name: name.trim(),
			popup_type: popupType,
			trigger_type: triggerType,
			trigger_config: triggerConfig,
			content_json: { elements },
			style_json: {
				background: bg,
				textColor,
				accentColor,
				borderRadius,
				maxWidth,
				animation,
				backdrop: true,
				backdropColor: 'rgba(0,0,0,0.5)'
			},
			targeting_rules: { pages, devices, userStatus },
			display_frequency: frequency,
			starts_at: startsAt || undefined,
			expires_at: expiresAt || undefined,
			is_active: false,
			priority: 0
		};
		try {
			await api.post('/api/admin/popups', payload);
			goto('/admin/popups');
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to create popup';
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head><title>New Popup - Admin</title></svelte:head>

<div class="pb">
	<a href="/admin/popups" class="pb__back"><ArrowLeftIcon size={18} /> Back to Popups</a>

	<div class="pb__top">
		<div class="pb__top-left">
			<label class="pb__visually-hidden" for="popup-name">Popup name</label>
			<input
				id="popup-name"
				name="popup-name"
				type="text"
				bind:value={name}
				class="pb__name"
				placeholder="Popup name..."
			/>
			<label class="pb__visually-hidden" for="popup-type">Popup type</label>
			<select id="popup-type" name="popup-type" bind:value={popupType} class="pb__select">
				{#each popupTypes as pt (pt.value)}
					<option value={pt.value}>{pt.label}</option>
				{/each}
			</select>
		</div>
		<button class="pb__save" disabled={saving} onclick={save}>
			<FloppyDiskIcon size={18} weight="bold" />
			{saving ? 'Saving...' : 'Save Popup'}
		</button>
	</div>

	{#if error}
		<div class="pb__error">{error}</div>
	{/if}

	<div class="pb__body">
		<div class="pb__main">
			<!-- Config Tabs -->
			<div class="pb__tabs">
				{#each ['trigger', 'targeting', 'style', 'display'] as tab (tab)}
					<button
						class="pb__tab"
						class:pb__tab--active={activeTab === tab}
						onclick={() => (activeTab = tab as typeof activeTab)}
					>
						{formatType(tab)}
					</button>
				{/each}
			</div>

			<div class="pb__panel">
				{#if activeTab === 'trigger'}
					<div class="pb__field">
						<label class="pb__label" for="trigger-type">Trigger Type</label>
						<select
							id="trigger-type"
							name="trigger-type"
							bind:value={triggerType}
							class="pb__input"
						>
							{#each triggerTypes as tt (tt.value)}
								<option value={tt.value}>{tt.label}</option>
							{/each}
						</select>
					</div>
					{#if triggerType === 'time_delay'}
						<div class="pb__field">
							<label class="pb__label" for="trigger-delay-ms">Delay (ms)</label>
							<input
								id="trigger-delay-ms"
								name="trigger-delay-ms"
								type="number"
								min="0"
								step="100"
								bind:value={triggerDelayMs}
								class="pb__input"
							/>
						</div>
					{/if}
					{#if triggerType === 'scroll_percentage'}
						<div class="pb__field">
							<label class="pb__label" for="trigger-scroll-pct"
								>Scroll Percentage</label
							>
							<input
								id="trigger-scroll-pct"
								name="trigger-scroll-pct"
								type="number"
								min="0"
								max="100"
								bind:value={triggerScrollPct}
								class="pb__input"
							/>
						</div>
					{/if}
				{:else if activeTab === 'targeting'}
					<div class="pb__field">
						<label class="pb__label" for="targeting-new-page">Page Patterns</label>
						<div class="pb__tag-list">
							{#each pages as p (p)}
								<span class="pb__tag"
									>{p}
									<button class="pb__tag-x" onclick={() => removePage(p)}
										>x</button
									></span
								>
							{/each}
						</div>
						<div class="pb__row">
							<input
								id="targeting-new-page"
								name="targeting-new-page"
								type="text"
								bind:value={newPage}
								class="pb__input"
								placeholder="/pricing, /blog/*"
								onkeydown={(e) =>
									e.key === 'Enter' && (e.preventDefault(), addPage())}
							/>
							<button class="pb__btn-sm" onclick={addPage}
								><PlusIcon size={14} /></button
							>
						</div>
					</div>
					<div class="pb__field">
						<span class="pb__label">Devices</span>
						<div class="pb__checks">
							{#each ['desktop', 'mobile', 'tablet'] as d (d)}
								<label class="pb__check" for={`targeting-device-${d}`}
									><input
										id={`targeting-device-${d}`}
										name={`targeting-device-${d}`}
										type="checkbox"
										checked={devices.includes(d as 'desktop')}
										onchange={() => toggleDevice(d as 'desktop')}
									/>{formatType(d)}</label
								>
							{/each}
						</div>
					</div>
					<div class="pb__field">
						<span class="pb__label">User Status</span>
						<div class="pb__checks">
							{#each ['all', 'logged_in', 'logged_out', 'member', 'non_member'] as s (s)}
								<label class="pb__check" for={`targeting-user-${s}`}
									><input
										id={`targeting-user-${s}`}
										name={`targeting-user-${s}`}
										type="checkbox"
										checked={userStatus.includes(s as 'all')}
										onchange={() => toggleUserStatus(s as 'all')}
									/>{formatType(s)}</label
								>
							{/each}
						</div>
					</div>
				{:else if activeTab === 'style'}
					<div class="pb__grid2">
						<div class="pb__field">
							<label class="pb__label" for="style-bg-color">Background</label>
							<div class="pb__color-row">
								<input
									id="style-bg-color"
									name="style-bg-color"
									type="color"
									bind:value={bg}
									class="pb__color"
								/><input
									id="style-bg-hex"
									name="style-bg-hex"
									type="text"
									aria-label="Background hex value"
									bind:value={bg}
									class="pb__input pb__input--sm"
								/>
							</div>
						</div>
						<div class="pb__field">
							<label class="pb__label" for="style-text-color">Text Color</label>
							<div class="pb__color-row">
								<input
									id="style-text-color"
									name="style-text-color"
									type="color"
									bind:value={textColor}
									class="pb__color"
								/><input
									id="style-text-hex"
									name="style-text-hex"
									type="text"
									aria-label="Text color hex value"
									bind:value={textColor}
									class="pb__input pb__input--sm"
								/>
							</div>
						</div>
						<div class="pb__field">
							<label class="pb__label" for="style-accent-color">Accent Color</label>
							<div class="pb__color-row">
								<input
									id="style-accent-color"
									name="style-accent-color"
									type="color"
									bind:value={accentColor}
									class="pb__color"
								/><input
									id="style-accent-hex"
									name="style-accent-hex"
									type="text"
									aria-label="Accent color hex value"
									bind:value={accentColor}
									class="pb__input pb__input--sm"
								/>
							</div>
						</div>
						<div class="pb__field">
							<label class="pb__label" for="style-border-radius">Border Radius</label>
							<input
								id="style-border-radius"
								name="style-border-radius"
								type="text"
								bind:value={borderRadius}
								class="pb__input"
							/>
						</div>
						<div class="pb__field">
							<label class="pb__label" for="style-animation">Animation</label>
							<select
								id="style-animation"
								name="style-animation"
								bind:value={animation}
								class="pb__input"
							>
								{#each animations as a (a)}<option value={a}>{formatType(a)}</option
									>{/each}
							</select>
						</div>
						<div class="pb__field">
							<label class="pb__label" for="style-max-width">Max Width</label>
							<input
								id="style-max-width"
								name="style-max-width"
								type="text"
								bind:value={maxWidth}
								class="pb__input"
							/>
						</div>
					</div>
				{:else if activeTab === 'display'}
					<div class="pb__field">
						<label class="pb__label" for="display-frequency">Frequency</label>
						<select
							id="display-frequency"
							name="display-frequency"
							bind:value={frequency}
							class="pb__input"
						>
							{#each frequencies as f (f.value)}<option value={f.value}>{f.label}</option
								>{/each}
						</select>
					</div>
					<div class="pb__grid2">
						<div class="pb__field">
							<label class="pb__label" for="display-starts-at">Start Date</label>
							<input
								id="display-starts-at"
								name="display-starts-at"
								type="datetime-local"
								bind:value={startsAt}
								class="pb__input"
							/>
						</div>
						<div class="pb__field">
							<label class="pb__label" for="display-expires-at">End Date</label>
							<input
								id="display-expires-at"
								name="display-expires-at"
								type="datetime-local"
								bind:value={expiresAt}
								class="pb__input"
							/>
						</div>
					</div>
				{/if}
			</div>

			<!-- Content / Elements -->
			<div class="pb__section-head">
				<h2 class="pb__section-title">Content Elements</h2>
			</div>
			<div class="pb__add-btns">
				<button class="pb__add" onclick={() => addElement('heading')}
					><TextHIcon size={16} /> Heading</button
				>
				<button class="pb__add" onclick={() => addElement('text')}
					><TextAaIcon size={16} /> Text</button
				>
				<button class="pb__add" onclick={() => addElement('email')}
					><EnvelopeSimpleIcon size={16} /> Email</button
				>
				<button class="pb__add" onclick={() => addElement('input')}
					><TextboxIcon size={16} /> Input</button
				>
				<button class="pb__add" onclick={() => addElement('button')}
					><CursorClickIcon size={16} /> Button</button
				>
				<button class="pb__add" onclick={() => addElement('divider')}
					><LineSegmentIcon size={16} /> Divider</button
				>
			</div>

			{#if elements.length === 0}
				<p class="pb__empty">No elements yet. Add some above.</p>
			{:else}
				<div class="pb__elements">
					{#each elements as el, i (el.id)}
						<div class="pb__el">
							<div class="pb__el-head">
								<span class="pb__el-type">{formatType(el.type)}</span>
								<span class="pb__el-preview">{previewText(el)}</span>
								<div class="pb__el-actions">
									<button
										class="pb__el-btn"
										onclick={() => moveElement(el.id, -1)}
										disabled={i === 0}><CaretUpIcon size={14} /></button
									>
									<button
										class="pb__el-btn"
										onclick={() => moveElement(el.id, 1)}
										disabled={i === elements.length - 1}
										><CaretDownIcon size={14} /></button
									>
									<button
										class="pb__el-btn"
										onclick={() =>
											(editingId = editingId === el.id ? null : el.id)}
										><PencilSimpleIcon size={14} /></button
									>
									<button
										class="pb__el-btn pb__el-btn--del"
										onclick={() => removeElement(el.id)}
										><TrashIcon size={14} /></button
									>
								</div>
							</div>
							{#if editingId === el.id}
								<div class="pb__el-edit">
									{#if el.type === 'heading' || el.type === 'text'}
										<div class="pb__field">
											<label class="pb__label" for={`el-${el.id}-text`}
												>Text</label
											><input
												id={`el-${el.id}-text`}
												name={`el-${el.id}-text`}
												type="text"
												value={el.props.text ?? ''}
												oninput={(e) =>
													updateProp(
														el.id,
														'text',
														e.currentTarget.value
													)}
												class="pb__input"
											/>
										</div>
									{/if}
									{#if el.type === 'heading'}
										<div class="pb__field">
											<label class="pb__label" for={`el-${el.id}-level`}
												>Level</label
											>
											<select
												id={`el-${el.id}-level`}
												name={`el-${el.id}-level`}
												value={el.props.level ?? 'h2'}
												onchange={(e) =>
													updateProp(
														el.id,
														'level',
														e.currentTarget.value
													)}
												class="pb__input"
											>
												<option value="h1">H1</option><option value="h2"
													>H2</option
												><option value="h3">H3</option>
											</select>
										</div>
									{/if}
									{#if el.type === 'email' || el.type === 'input'}
										<div class="pb__field">
											<label class="pb__label" for={`el-${el.id}-label`}
												>Label</label
											><input
												id={`el-${el.id}-label`}
												name={`el-${el.id}-label`}
												type="text"
												value={el.props.label ?? ''}
												oninput={(e) =>
													updateProp(
														el.id,
														'label',
														e.currentTarget.value
													)}
												class="pb__input"
											/>
										</div>
										<div class="pb__field">
											<label class="pb__label" for={`el-${el.id}-placeholder`}
												>Placeholder</label
											><input
												id={`el-${el.id}-placeholder`}
												name={`el-${el.id}-placeholder`}
												type="text"
												value={el.props.placeholder ?? ''}
												oninput={(e) =>
													updateProp(
														el.id,
														'placeholder',
														e.currentTarget.value
													)}
												class="pb__input"
											/>
										</div>
										<label class="pb__check" for={`el-${el.id}-required`}
											><input
												id={`el-${el.id}-required`}
												name={`el-${el.id}-required`}
												type="checkbox"
												checked={!!el.props.required}
												onchange={() =>
													updateProp(
														el.id,
														'required',
														!el.props.required
													)}
											/> Required</label
										>
									{/if}
									{#if el.type === 'button'}
										<div class="pb__field">
											<label class="pb__label" for={`el-${el.id}-btn-text`}
												>Button Text</label
											><input
												id={`el-${el.id}-btn-text`}
												name={`el-${el.id}-btn-text`}
												type="text"
												value={el.props.text ?? ''}
												oninput={(e) =>
													updateProp(
														el.id,
														'text',
														e.currentTarget.value
													)}
												class="pb__input"
											/>
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Preview -->
		<div class="pb__preview-wrap">
			<div class="pb__preview-head"><EyeIcon size={16} /> Preview</div>
			<div
				class="pb__preview-box"
				style:background={bg}
				style:color={textColor}
				style:border-radius={borderRadius}
				style:max-width={maxWidth}
			>
				{#each elements as el (el.id)}
					{#if el.type === 'heading'}
						<div class="pv-heading" style:color={textColor}>
							{el.props.text ?? 'Heading'}
						</div>
					{:else if el.type === 'text'}
						<p class="pv-text" style:color={textColor}>{el.props.text ?? ''}</p>
					{:else if el.type === 'email' || el.type === 'input'}
						<div class="pv-field">
							{#if el.props.label}<span class="pv-label" style:color={textColor}
									>{el.props.label}</span
								>{/if}
							<div class="pv-input">{el.props.placeholder ?? ''}</div>
						</div>
					{:else if el.type === 'button'}
						<div class="pv-btn" style:background={accentColor}>
							{el.props.text ?? 'Button'}
						</div>
					{:else if el.type === 'divider'}
						<hr class="pv-divider" />
					{/if}
				{/each}
				{#if elements.length === 0}
					<p class="pv-empty">Preview will appear here</p>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
	.pb {
		max-width: var(--container-max);
	}
	.pb__visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
	.pb__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.25rem;
		transition: color 200ms var(--ease-out);
	}
	.pb__back:hover {
		color: var(--color-white);
	}
	.pb__top {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}
	.pb__top-left {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		flex: 1;
		min-width: 0;
	}
	.pb__name {
		flex: 1;
		min-width: 180px;
		padding: 0.6rem 0.85rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-md);
		font-family: var(--font-heading);
		font-weight: var(--w-semibold);
	}
	.pb__name:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.pb__name::placeholder {
		color: var(--color-grey-500);
	}
	.pb__select {
		padding: 0.6rem 0.85rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
	}
	.pb__select:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.pb__save {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-2xl);
		border: none;
		cursor: pointer;
		transition: opacity 200ms;
	}
	.pb__save:hover:not(:disabled) {
		opacity: 0.9;
	}
	.pb__save:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.pb__error {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.65rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}
	.pb__body {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.pb__main {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.pb__tabs {
		display: flex;
		gap: 0.25rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		margin-bottom: 0;
	}
	.pb__tab {
		padding: 0.55rem 1rem;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
		transition:
			color 200ms,
			border-color 200ms;
	}
	.pb__tab:hover {
		color: var(--color-grey-200);
	}
	.pb__tab--active {
		color: var(--color-teal);
		border-bottom-color: var(--color-teal);
	}
	.pb__panel {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 0 0 var(--radius-xl) var(--radius-xl);
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.pb__field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.pb__label {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
	}
	.pb__input {
		width: 100%;
		padding: 0.55rem 0.75rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
	}
	.pb__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.pb__input::placeholder {
		color: var(--color-grey-500);
	}
	.pb__input--sm {
		flex: 1;
		min-width: 0;
	}
	.pb__grid2 {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}
	.pb__color-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.pb__color {
		width: 2rem;
		height: 2rem;
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: var(--radius-md);
		cursor: pointer;
		padding: 0;
		background: none;
	}
	.pb__row {
		display: flex;
		gap: 0.5rem;
	}
	.pb__btn-sm {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		background: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-2xl);
		color: var(--color-teal);
		cursor: pointer;
		flex-shrink: 0;
	}
	.pb__tag-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
	}
	.pb__tag {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.2rem 0.6rem;
		background: rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
	}
	.pb__tag-x {
		background: none;
		border: none;
		color: var(--color-grey-400);
		cursor: pointer;
		font-size: var(--fs-xs);
		padding: 0 0.15rem;
		line-height: 1;
	}
	.pb__checks {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}
	.pb__check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		cursor: pointer;
	}
	.pb__check input {
		accent-color: var(--color-teal);
	}
	.pb__section-head {
		margin-top: 0.5rem;
	}
	.pb__section-title {
		font-size: var(--fs-md);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}
	.pb__add-btns {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}
	.pb__add {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.4rem 0.75rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		cursor: pointer;
		transition:
			border-color 200ms,
			background 200ms;
	}
	.pb__add:hover {
		border-color: var(--color-teal);
		background: rgba(15, 164, 175, 0.08);
		color: var(--color-teal);
	}
	.pb__empty {
		text-align: center;
		padding: 2rem;
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
	}
	.pb__elements {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.pb__el {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		overflow: hidden;
	}
	.pb__el-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 0.75rem;
	}
	.pb__el-type {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-teal);
		background: rgba(15, 164, 175, 0.1);
		padding: 0.1rem 0.5rem;
		border-radius: var(--radius-full);
		white-space: nowrap;
	}
	.pb__el-preview {
		flex: 1;
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}
	.pb__el-actions {
		display: flex;
		gap: 0.25rem;
		flex-shrink: 0;
	}
	.pb__el-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 1.6rem;
		height: 1.6rem;
		background: rgba(255, 255, 255, 0.04);
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-grey-400);
		cursor: pointer;
		transition:
			background 150ms,
			color 150ms;
	}
	.pb__el-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
		color: var(--color-white);
	}
	.pb__el-btn:disabled {
		opacity: 0.3;
		cursor: default;
	}
	.pb__el-btn--del:hover {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-red);
	}
	.pb__el-edit {
		padding: 0.75rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		background: rgba(0, 0, 0, 0.15);
	}
	/* Preview */
	.pb__preview-wrap {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		overflow: hidden;
	}
	.pb__preview-head {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 1rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-grey-300);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.pb__preview-box {
		padding: 1.5rem;
		margin: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		border: 1px dashed rgba(255, 255, 255, 0.1);
		min-height: 120px;
	}
	.pv-heading {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
	}
	.pv-text {
		font-size: var(--fs-sm);
		line-height: var(--lh-normal);
		margin: 0;
	}
	.pv-field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.pv-label {
		font-size: var(--fs-xs);
		opacity: 0.8;
	}
	.pv-input {
		padding: 0.5rem 0.65rem;
		background: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		font-size: var(--fs-sm);
		opacity: 0.6;
	}
	.pv-btn {
		padding: 0.6rem 1.25rem;
		border-radius: var(--radius-2xl);
		color: #fff;
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		text-align: center;
	}
	.pv-divider {
		border: none;
		border-top: 1px solid rgba(255, 255, 255, 0.15);
		margin: 0.25rem 0;
	}
	.pv-empty {
		text-align: center;
		opacity: 0.4;
		font-size: var(--fs-sm);
		margin: auto;
	}

	@media (min-width: 768px) {
		.pb__body {
			flex-direction: row;
			align-items: flex-start;
		}
		.pb__main {
			flex: 1;
			min-width: 0;
		}
		.pb__preview-wrap {
			width: 340px;
			flex-shrink: 0;
			position: sticky;
			top: 1rem;
		}
		.pb__grid2 {
			grid-template-columns: 1fr 1fr;
		}
	}

	@media (max-width: 767px) {
		.pb__grid2 {
			grid-template-columns: 1fr;
		}
	}
</style>
