<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import type {
		Popup, PopupType, PopupTrigger, PopupFrequency,
		PopupElement, PopupStyle, PopupTargetingRules, UpdatePopupPayload, PopupAnalytics
	} from '$lib/api/types';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	const popupId = $derived(page.params.id);

	let loading = $state(true);
	let name = $state('');
	let popupType = $state<PopupType>('modal');
	let triggerType = $state<PopupTrigger>('on_load');
	let triggerDelayMs = $state(0);
	let triggerScrollPct = $state(50);
	let pages_ = $state<string[]>(['*']);
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
	let activeTab = $state<'trigger' | 'targeting' | 'style' | 'display' | 'analytics'>('trigger');
	let saving = $state(false);
	let deleting = $state(false);
	let error = $state('');
	let analytics = $state<PopupAnalytics | null>(null);

	onMount(async () => {
		try {
			const p = await api.get<Popup & { impressions?: number; submissions?: number; conversion_rate?: number }>(`/api/admin/popups/${popupId}`);
			name = p.name;
			popupType = p.popup_type;
			triggerType = p.trigger_type;
			triggerDelayMs = (p.trigger_config as Record<string, number>).delay_ms ?? 0;
			triggerScrollPct = (p.trigger_config as Record<string, number>).scroll_pct ?? 50;
			pages_ = p.targeting_rules.pages ?? ['*'];
			devices = p.targeting_rules.devices ?? ['desktop', 'mobile', 'tablet'];
			userStatus = p.targeting_rules.userStatus ?? ['all'];
			bg = p.style_json.background ?? '#132b50';
			textColor = p.style_json.textColor ?? '#ffffff';
			accentColor = p.style_json.accentColor ?? '#0fa4af';
			borderRadius = p.style_json.borderRadius ?? '12px';
			animation = p.style_json.animation ?? 'fade';
			maxWidth = p.style_json.maxWidth ?? '480px';
			frequency = p.display_frequency;
			startsAt = p.starts_at?.slice(0, 16) ?? '';
			expiresAt = p.expires_at?.slice(0, 16) ?? '';
			elements = p.content_json?.elements ?? [];
			analytics = {
				popup_id: p.id,
				popup_name: p.name,
				total_impressions: p.impressions ?? 0,
				total_closes: 0,
				total_submissions: p.submissions ?? 0,
				conversion_rate: p.conversion_rate ?? 0
			};
		} catch { error = 'Failed to load popup'; }
		finally { loading = false; }
	});

	function uid(): string { return Math.random().toString(36).slice(2, 10); }

	function addElement(type: PopupElement['type']) {
		const defaults: Record<string, Record<string, unknown>> = {
			heading: { text: 'Heading', level: 'h2' },
			text: { text: 'Your message here.' },
			email: { placeholder: 'Enter your email', label: 'Email', required: true },
			input: { placeholder: 'Enter text', label: 'Text', required: false },
			button: { text: 'Submit', action: 'submit' },
			divider: {},
		};
		elements = [...elements, { id: uid(), type, props: defaults[type] ?? {} }];
	}

	function removeElement(id: string) { elements = elements.filter(e => e.id !== id); }
	function moveUp(i: number) { if (i > 0) { const a = [...elements]; [a[i-1], a[i]] = [a[i], a[i-1]]; elements = a; } }
	function moveDown(i: number) { if (i < elements.length - 1) { const a = [...elements]; [a[i], a[i+1]] = [a[i+1], a[i]]; elements = a; } }

	function removePage(p: string) { pages_ = pages_.filter(x => x !== p); }
	function addPage() { if (newPage.trim()) { pages_ = [...pages_, newPage.trim()]; newPage = ''; } }
	function toggleDevice(d: 'desktop'|'mobile'|'tablet') { devices = devices.includes(d) ? devices.filter(x=>x!==d) as typeof devices : [...devices, d]; }

	async function handleSave() {
		if (!name.trim()) { error = 'Name is required'; return; }
		saving = true; error = '';
		const triggerConfig: Record<string, unknown> = {};
		if (triggerType === 'time_delay') triggerConfig.delay_ms = triggerDelayMs;
		if (triggerType === 'scroll_percentage') triggerConfig.scroll_pct = triggerScrollPct;

		const payload: UpdatePopupPayload = {
			name, popup_type: popupType, trigger_type: triggerType,
			trigger_config: triggerConfig,
			content_json: { elements },
			style_json: { background: bg, textColor, accentColor, borderRadius, maxWidth, animation, backdrop: true, backdropColor: 'rgba(0,0,0,0.6)' },
			targeting_rules: { pages: pages_, devices, userStatus },
			display_frequency: frequency,
			starts_at: startsAt || undefined, expires_at: expiresAt || undefined
		};
		try {
			await api.put(`/api/admin/popups/${popupId}`, payload);
			error = '';
			// Stay on page
		} catch (e) { error = e instanceof ApiError ? e.message : 'Failed to update'; }
		finally { saving = false; }
	}

	async function handleDelete() {
		const ok = await confirmDialog({
			title: 'Delete this popup?',
			message: 'The popup, its targeting rules, and its analytics history will be permanently removed.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		deleting = true;
		try { await api.del(`/api/admin/popups/${popupId}`); goto('/admin/popups'); }
		catch { error = 'Failed to delete'; deleting = false; }
	}
</script>

<svelte:head><title>Edit Popup - Admin</title></svelte:head>

<div class="popup-edit">
	{#if loading}
		<p class="popup-edit__loading">Loading popup...</p>
	{:else}
		<div class="popup-edit__header">
			<a href="/admin/popups" class="popup-edit__back"><ArrowLeftIcon size={18} /> Back</a>
			<input type="text" bind:value={name} placeholder="Popup name" class="popup-edit__name" />
			<select bind:value={popupType} class="popup-edit__select">
				<option value="modal">Modal</option>
				<option value="slide_in">Slide In</option>
				<option value="banner">Banner</option>
				<option value="fullscreen">Fullscreen</option>
				<option value="floating_bar">Floating Bar</option>
			</select>
			<button class="popup-edit__save" onclick={handleSave} disabled={saving}>
				<FloppyDiskIcon size={16} /> {saving ? 'Saving...' : 'Update'}
			</button>
			<button class="popup-edit__delete" onclick={handleDelete} disabled={deleting}>
				<TrashIcon size={16} /> {deleting ? 'Deleting...' : 'Delete'}
			</button>
		</div>

		{#if error}<div class="popup-edit__error">{error}</div>{/if}

		<!-- Analytics -->
		{#if analytics}
			<div class="popup-edit__analytics">
				<div class="stat"><span class="stat__value">{analytics.total_impressions}</span><span class="stat__label">Impressions</span></div>
				<div class="stat"><span class="stat__value">{analytics.total_submissions}</span><span class="stat__label">Submissions</span></div>
				<div class="stat"><span class="stat__value">{(analytics.conversion_rate * 100).toFixed(1)}%</span><span class="stat__label">Conversion</span></div>
			</div>
		{/if}

		<!-- Tabs -->
		<div class="popup-edit__tabs">
			{#each ['trigger', 'targeting', 'style', 'display'] as tab}
				<button class="tab" class:tab--active={activeTab === tab} onclick={() => (activeTab = tab as typeof activeTab)}>{tab}</button>
			{/each}
		</div>

		<div class="popup-edit__config">
			{#if activeTab === 'trigger'}
				<label class="field"><span>Trigger</span>
					<select bind:value={triggerType} class="field__input">
						<option value="on_load">On Load</option><option value="exit_intent">Exit Intent</option>
						<option value="scroll_percentage">Scroll %</option><option value="time_delay">Time Delay</option>
						<option value="click">Click</option><option value="inactivity">Inactivity</option>
					</select>
				</label>
				{#if triggerType === 'time_delay'}
					<label class="field"><span>Delay (ms)</span><input type="number" bind:value={triggerDelayMs} class="field__input" /></label>
				{/if}
				{#if triggerType === 'scroll_percentage'}
					<label class="field"><span>Scroll %</span><input type="range" min="0" max="100" bind:value={triggerScrollPct} /><span>{triggerScrollPct}%</span></label>
				{/if}
			{:else if activeTab === 'targeting'}
				<div class="field"><span>Pages</span>
					{#each pages_ as p}<span class="chip">{p} <button onclick={() => removePage(p)}>&times;</button></span>{/each}
					<div class="field__row"><input type="text" bind:value={newPage} placeholder="/blog/*" class="field__input" /><button class="field__btn" onclick={addPage}><PlusIcon size={14} /></button></div>
				</div>
				<div class="field"><span>Devices</span>
					{#each (['desktop','mobile','tablet'] as const) as d}<label class="checkbox"><input type="checkbox" checked={devices.includes(d)} onchange={() => toggleDevice(d)} /> {d}</label>{/each}
				</div>
			{:else if activeTab === 'style'}
				<div class="field__grid">
					<label class="field"><span>Background</span><input type="color" bind:value={bg} /></label>
					<label class="field"><span>Text</span><input type="color" bind:value={textColor} /></label>
					<label class="field"><span>Accent</span><input type="color" bind:value={accentColor} /></label>
					<label class="field"><span>Radius</span><input type="text" bind:value={borderRadius} class="field__input" /></label>
					<label class="field"><span>Max Width</span><input type="text" bind:value={maxWidth} class="field__input" /></label>
					<label class="field"><span>Animation</span>
						<select bind:value={animation} class="field__input">
							{#each ['fade','slide_up','slide_down','scale','none'] as a}<option value={a}>{a}</option>{/each}
						</select>
					</label>
				</div>
			{:else if activeTab === 'display'}
				<label class="field"><span>Frequency</span>
					<select bind:value={frequency} class="field__input">
						<option value="every_time">Every Time</option><option value="once_per_session">Once Per Session</option>
						<option value="once_ever">Once Ever</option>
					</select>
				</label>
				<label class="field"><span>Starts</span><input type="datetime-local" bind:value={startsAt} class="field__input" /></label>
				<label class="field"><span>Expires</span><input type="datetime-local" bind:value={expiresAt} class="field__input" /></label>
			{/if}
		</div>

		<!-- Elements -->
		<div class="popup-edit__elements">
			<h3>Content Elements</h3>
			<div class="popup-edit__add-bar">
				{#each [['heading','H'],['text','T'],['email','@'],['input','Aa'],['button','Btn'],['divider','—']] as [type, label]}
					<button class="add-btn" onclick={() => addElement(type as PopupElement['type'])}><PlusIcon size={12} /> {label}</button>
				{/each}
			</div>
			{#each elements as el, i (el.id)}
				<div class="element" class:element--editing={editingId === el.id}>
					<div class="element__row">
						<span class="element__type">{el.type}</span>
						<span class="element__preview">{String(el.props.text ?? el.props.label ?? '')}</span>
						<button onclick={() => (editingId = editingId === el.id ? null : el.id)}><PencilSimpleIcon size={14} /></button>
						<button onclick={() => moveUp(i)}><CaretUpIcon size={14} /></button>
						<button onclick={() => moveDown(i)}><CaretDownIcon size={14} /></button>
						<button onclick={() => removeElement(el.id)}><TrashIcon size={14} /></button>
					</div>
					{#if editingId === el.id}
						<div class="element__edit">
							{#if el.type === 'heading' || el.type === 'text' || el.type === 'button'}
								<label class="field"><span>Text</span><input type="text" bind:value={el.props.text} class="field__input" /></label>
							{/if}
							{#if el.type === 'email' || el.type === 'input'}
								<label class="field"><span>Label</span><input type="text" bind:value={el.props.label} class="field__input" /></label>
								<label class="field"><span>Placeholder</span><input type="text" bind:value={el.props.placeholder} class="field__input" /></label>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Preview -->
		<div class="popup-edit__preview">
			<h3><EyeIcon size={16} /> Preview</h3>
			<div class="preview-box" style="background:{bg}; color:{textColor}; border-radius:{borderRadius}; max-width:{maxWidth}; padding:1.5rem;">
				{#each elements as el}
					{#if el.type === 'heading'}<h2 style="margin:0 0 0.5rem">{el.props.text ?? 'Heading'}</h2>
					{:else if el.type === 'text'}<p style="margin:0 0 0.5rem; opacity:0.8; font-size:0.9rem">{el.props.text ?? ''}</p>
					{:else if el.type === 'email' || el.type === 'input'}<div style="margin:0 0 0.5rem"><span style="font-size:0.75rem; opacity:0.7">{el.props.label ?? ''}</span><input type="text" placeholder={String(el.props.placeholder ?? '')} style="width:100%; padding:0.5rem; background:rgba(255,255,255,0.1); border:1px solid rgba(255,255,255,0.2); border-radius:6px; color:{textColor}" /></div>
					{:else if el.type === 'button'}<button style="padding:0.6rem 1.5rem; background:{accentColor}; color:white; border:none; border-radius:6px; font-weight:600; cursor:pointer; width:100%">{el.props.text ?? 'Submit'}</button>
					{:else if el.type === 'divider'}<hr style="border:none; border-top:1px solid rgba(255,255,255,0.1); margin:0.75rem 0" />
					{/if}
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.popup-edit { padding: 1.5rem; max-width: 900px; }
	.popup-edit__loading { color: var(--color-grey-400); }
	.popup-edit__header { display: flex; flex-wrap: wrap; align-items: center; gap: 0.75rem; margin-bottom: 1.5rem; }
	.popup-edit__back { display: inline-flex; align-items: center; gap: 0.35rem; color: var(--color-teal); text-decoration: none; font-size: var(--fs-sm); }
	.popup-edit__name { flex: 1; min-width: 200px; padding: 0.6rem 1rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-2xl); color: var(--color-white); font-size: var(--fs-base); font-weight: var(--w-semibold); }
	.popup-edit__name:focus { outline: none; border-color: var(--color-teal); }
	.popup-edit__select { padding: 0.6rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-2xl); color: var(--color-white); font-size: var(--fs-sm); }
	.popup-edit__save { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.6rem 1rem; background: var(--color-teal); color: var(--color-white); border: none; border-radius: var(--radius-2xl); font-weight: var(--w-semibold); font-size: var(--fs-sm); cursor: pointer; }
	.popup-edit__save:disabled { opacity: 0.5; }
	.popup-edit__delete { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.6rem 1rem; background: rgba(239,68,68,0.15); color: #fca5a5; border: 1px solid rgba(239,68,68,0.3); border-radius: var(--radius-2xl); font-size: var(--fs-sm); cursor: pointer; }
	.popup-edit__error { background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.3); color: #fca5a5; padding: 0.65rem 1rem; border-radius: var(--radius-2xl); font-size: var(--fs-sm); margin-bottom: 1rem; }
	.popup-edit__analytics { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; margin-bottom: 1.5rem; }
	.stat { background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-2xl); padding: 1rem; text-align: center; }
	.stat__value { display: block; font-size: var(--fs-2xl); font-weight: var(--w-bold); color: var(--color-white); }
	.stat__label { font-size: var(--fs-xs); color: var(--color-grey-400); }
	.popup-edit__tabs { display: flex; gap: 0.25rem; margin-bottom: 1rem; background: rgba(255,255,255,0.03); border-radius: var(--radius-2xl); padding: 0.25rem; }
	.tab { flex: 1; padding: 0.5rem; background: none; border: none; color: var(--color-grey-400); font-size: var(--fs-sm); font-weight: var(--w-medium); cursor: pointer; border-radius: var(--radius-md); text-transform: capitalize; }
	.tab--active { background: rgba(15,164,175,0.15); color: var(--color-teal); }
	.popup-edit__config { background: rgba(255,255,255,0.02); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-2xl); padding: 1.25rem; margin-bottom: 1.5rem; }
	.field { display: flex; flex-direction: column; gap: 0.35rem; margin-bottom: 0.75rem; }
	.field span { font-size: var(--fs-xs); color: var(--color-grey-400); font-weight: var(--w-medium); }
	.field__input { padding: 0.5rem 0.75rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-md); color: var(--color-white); font-size: var(--fs-sm); }
	.field__input:focus { outline: none; border-color: var(--color-teal); }
	.field__grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
	.field__row { display: flex; gap: 0.35rem; }
	.field__btn { padding: 0.5rem; background: var(--color-teal); color: white; border: none; border-radius: var(--radius-md); cursor: pointer; }
	.chip { display: inline-flex; align-items: center; gap: 0.25rem; background: rgba(15,164,175,0.1); color: var(--color-teal); padding: 0.2rem 0.5rem; border-radius: var(--radius-full); font-size: var(--fs-xs); margin: 0.15rem; }
	.chip button { background: none; border: none; color: var(--color-teal); cursor: pointer; font-size: 1rem; line-height: 1; }
	.checkbox { display: inline-flex; align-items: center; gap: 0.35rem; color: var(--color-grey-300); font-size: var(--fs-sm); margin-right: 1rem; }
	.popup-edit__elements { margin-bottom: 1.5rem; }
	.popup-edit__elements h3 { font-size: var(--fs-base); font-weight: var(--w-semibold); color: var(--color-white); margin-bottom: 0.75rem; }
	.popup-edit__add-bar { display: flex; flex-wrap: wrap; gap: 0.35rem; margin-bottom: 0.75rem; }
	.add-btn { display: inline-flex; align-items: center; gap: 0.25rem; padding: 0.35rem 0.65rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-md); color: var(--color-grey-300); font-size: var(--fs-xs); cursor: pointer; }
	.add-btn:hover { background: rgba(255,255,255,0.1); }
	.element { background: rgba(255,255,255,0.02); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-md); padding: 0.5rem 0.75rem; margin-bottom: 0.35rem; }
	.element--editing { border-color: var(--color-teal); }
	.element__row { display: flex; align-items: center; gap: 0.5rem; }
	.element__type { font-size: var(--fs-xs); font-weight: var(--w-bold); color: var(--color-teal); text-transform: uppercase; min-width: 4rem; }
	.element__preview { flex: 1; font-size: var(--fs-sm); color: var(--color-grey-400); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	.element__row button { background: none; border: none; color: var(--color-grey-500); cursor: pointer; padding: 0.15rem; }
	.element__row button:hover { color: var(--color-white); }
	.element__edit { padding: 0.75rem 0 0; }
	.popup-edit__preview h3 { display: flex; align-items: center; gap: 0.5rem; font-size: var(--fs-base); font-weight: var(--w-semibold); color: var(--color-white); margin-bottom: 0.75rem; }
	.preview-box { border: 1px solid rgba(255,255,255,0.1); margin: 0 auto; }
</style>
