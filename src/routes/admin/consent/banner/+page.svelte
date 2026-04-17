<!--
  CONSENT-07 — banner configuration matrix.

  Lists every (region, locale) banner config, with inline create/edit via the
  Dialog primitive. The preview box renders the active banner config at
  whatever breakpoint the admin iframes into — the preview is intentionally
  minimal (title / body / button labels) so edits surface instantly without a
  round-trip.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog } from '$lib/components/shared';
	import {
		listBanners,
		createBanner,
		updateBanner,
		type AdminBannerConfig,
		type BannerUpsertBody
	} from '$lib/api/admin-consent';

	let banners = $state<AdminBannerConfig[]>([]);
	let loading = $state(true);
	let errorMsg = $state<string | null>(null);

	let editorOpen = $state(false);
	let editingId = $state<string | null>(null);

	let region = $state('default');
	let locale = $state('en');
	let layout = $state('bar');
	let position = $state('bottom');
	let isActive = $state(true);
	let copyTitle = $state('');
	let copyBody = $state('');
	let copyAcceptAll = $state('Accept all');
	let copyRejectAll = $state('Reject all');
	let copyCustomize = $state('Customize');
	let copySavePreferences = $state('Save preferences');
	let copyPrivacyHref = $state('/privacy');
	let copyPrivacyLabel = $state('Privacy policy');

	async function refresh() {
		loading = true;
		errorMsg = null;
		try {
			banners = await listBanners();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		void refresh();
	});

	function openCreate() {
		editingId = null;
		region = 'default';
		locale = 'en';
		layout = 'bar';
		position = 'bottom';
		isActive = true;
		copyTitle = 'We value your privacy';
		copyBody = '';
		copyAcceptAll = 'Accept all';
		copyRejectAll = 'Reject all';
		copyCustomize = 'Customize';
		copySavePreferences = 'Save preferences';
		copyPrivacyHref = '/privacy';
		copyPrivacyLabel = 'Privacy policy';
		editorOpen = true;
	}

	function openEdit(b: AdminBannerConfig) {
		editingId = b.id;
		region = b.region;
		locale = b.locale;
		layout = b.layout;
		position = b.position;
		isActive = b.is_active;
		const copy = b.copy_json as Record<string, string>;
		copyTitle = copy.title ?? '';
		copyBody = copy.body ?? '';
		copyAcceptAll = copy.acceptAll ?? 'Accept all';
		copyRejectAll = copy.rejectAll ?? 'Reject all';
		copyCustomize = copy.customize ?? 'Customize';
		copySavePreferences = copy.savePreferences ?? 'Save preferences';
		copyPrivacyHref = copy.privacyPolicyHref ?? '/privacy';
		copyPrivacyLabel = copy.privacyPolicyLabel ?? 'Privacy policy';
		editorOpen = true;
	}

	async function save() {
		const body: BannerUpsertBody = {
			region,
			locale,
			layout,
			position,
			is_active: isActive,
			copy_json: {
				title: copyTitle,
				body: copyBody,
				acceptAll: copyAcceptAll,
				rejectAll: copyRejectAll,
				customize: copyCustomize,
				savePreferences: copySavePreferences,
				privacyPolicyHref: copyPrivacyHref,
				privacyPolicyLabel: copyPrivacyLabel
			},
			theme_json: {}
		};
		try {
			if (editingId) {
				await updateBanner(editingId, body);
			} else {
				await createBanner(body);
			}
			editorOpen = false;
			await refresh();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		}
	}

	const breakpoints = [
		{ label: '320', width: 320 },
		{ label: '375', width: 375 },
		{ label: '480', width: 480 },
		{ label: '640', width: 640 },
		{ label: '768', width: 768 },
		{ label: '1024', width: 1024 },
		{ label: '1280', width: 1280 },
		{ label: '1536', width: 1536 },
		{ label: '1920', width: 1920 }
	] as const;

	let previewWidth = $state<number>(768);
</script>

<svelte:head>
	<title>Banner configs · Consent · Admin</title>
</svelte:head>

<header class="head">
	<h1>Banner configs</h1>
	<Button variant="primary" size="md" onclick={openCreate}>New banner</Button>
</header>

{#if errorMsg}
	<div class="error">{errorMsg}</div>
{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if banners.length === 0}
	<p class="muted">No banner configs yet. Create one to get started.</p>
{:else}
	<table class="table">
		<thead>
			<tr>
				<th>Region</th>
				<th>Locale</th>
				<th>Layout</th>
				<th>Position</th>
				<th>Version</th>
				<th>Active</th>
				<th></th>
			</tr>
		</thead>
		<tbody>
			{#each banners as b (b.id)}
				<tr>
					<td><code>{b.region}</code></td>
					<td><code>{b.locale}</code></td>
					<td>{b.layout}</td>
					<td>{b.position}</td>
					<td>v{b.version}</td>
					<td>{b.is_active ? 'yes' : 'no'}</td>
					<td><Button variant="tertiary" size="sm" onclick={() => openEdit(b)}>Edit</Button></td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}

<Dialog
	open={editorOpen}
	onclose={() => (editorOpen = false)}
	title={editingId ? 'Edit banner' : 'New banner'}
	size="lg"
>
	<div class="form">
		<div class="row">
			<label class="field">
				<span>Region</span>
				<input type="text" bind:value={region} />
			</label>
			<label class="field">
				<span>Locale</span>
				<input type="text" bind:value={locale} />
			</label>
		</div>
		<div class="row">
			<label class="field">
				<span>Layout</span>
				<select bind:value={layout}>
					<option value="bar">bar</option>
					<option value="box">box</option>
					<option value="popup">popup</option>
					<option value="fullscreen">fullscreen</option>
				</select>
			</label>
			<label class="field">
				<span>Position</span>
				<select bind:value={position}>
					<option value="bottom">bottom</option>
					<option value="top">top</option>
					<option value="center">center</option>
					<option value="bottom-start">bottom-start</option>
					<option value="bottom-end">bottom-end</option>
				</select>
			</label>
			<label class="field field--toggle">
				<input type="checkbox" bind:checked={isActive} />
				<span>Active</span>
			</label>
		</div>

		<label class="field">
			<span>Title</span>
			<input type="text" bind:value={copyTitle} />
		</label>
		<label class="field">
			<span>Body</span>
			<textarea rows="3" bind:value={copyBody}></textarea>
		</label>
		<div class="row">
			<label class="field">
				<span>Accept all label</span>
				<input type="text" bind:value={copyAcceptAll} />
			</label>
			<label class="field">
				<span>Reject all label</span>
				<input type="text" bind:value={copyRejectAll} />
			</label>
		</div>
		<div class="row">
			<label class="field">
				<span>Customize label</span>
				<input type="text" bind:value={copyCustomize} />
			</label>
			<label class="field">
				<span>Save preferences label</span>
				<input type="text" bind:value={copySavePreferences} />
			</label>
		</div>
		<div class="row">
			<label class="field">
				<span>Privacy policy href</span>
				<input type="text" bind:value={copyPrivacyHref} />
			</label>
			<label class="field">
				<span>Privacy policy label</span>
				<input type="text" bind:value={copyPrivacyLabel} />
			</label>
		</div>

		<!-- Live preview -->
		<section class="preview">
			<header>
				<strong>Preview</strong>
				<div class="bp-row">
					{#each breakpoints as bp (bp.width)}
						<button
							type="button"
							class="bp"
							class:bp--active={previewWidth === bp.width}
							onclick={() => (previewWidth = bp.width)}
						>
							{bp.label}
						</button>
					{/each}
				</div>
			</header>
			<div class="preview-frame" style="inline-size: {previewWidth}px">
				<div class="preview-banner" data-layout={layout}>
					<h3>{copyTitle || 'Banner title'}</h3>
					<p>{copyBody || 'Banner body copy.'}</p>
					<div class="preview-actions">
						<button type="button" class="preview-btn preview-btn--tertiary">
							{copyCustomize}
						</button>
						<button type="button" class="preview-btn preview-btn--secondary">
							{copyRejectAll}
						</button>
						<button type="button" class="preview-btn preview-btn--primary">
							{copyAcceptAll}
						</button>
					</div>
				</div>
			</div>
		</section>
	</div>
	{#snippet footer()}
		<Button variant="tertiary" onclick={() => (editorOpen = false)}>Cancel</Button>
		<Button variant="primary" onclick={save}>Save</Button>
	{/snippet}
</Dialog>

<style>
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		margin-block-end: var(--space-5);
	}
	.head h1 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
	}

	.error {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		border: 1px solid var(--surface-border-subtle);
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		margin-block-end: var(--space-4);
	}
	.muted {
		color: var(--surface-fg-muted);
	}

	.table {
		inline-size: 100%;
		border-collapse: collapse;
	}
	.table th,
	.table td {
		padding: var(--space-3) var(--space-3);
		border-block-end: 1px solid var(--surface-border-subtle);
		font-size: var(--fs-sm);
		text-align: start;
	}
	.table th {
		font-weight: var(--w-semibold);
		color: var(--surface-fg-muted);
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.row {
		display: flex;
		gap: var(--space-4);
		flex-wrap: wrap;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
		flex: 1 1 14rem;
		font-size: var(--fs-sm);
	}
	.field span {
		color: var(--surface-fg-muted);
	}
	.field--toggle {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
	}
	.field input[type='text'],
	.field select,
	.field textarea {
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		font: inherit;
	}

	.preview {
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--space-4);
	}
	.preview header {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		justify-content: space-between;
		margin-block-end: var(--space-3);
	}
	.bp-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.bp {
		padding: var(--space-1) var(--space-2);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-sm);
		background: transparent;
		font-size: var(--fs-xs);
		cursor: pointer;
	}
	.bp--active {
		background-color: var(--brand-teal-100);
		border-color: var(--brand-teal-500);
	}
	.preview-frame {
		margin: 0 auto;
		border: 1px dashed var(--surface-border-subtle);
		padding: var(--space-3);
		max-inline-size: 100%;
		transition: inline-size var(--duration-150) var(--ease-out);
	}
	.preview-banner {
		padding: var(--space-4);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
		box-shadow: var(--shadow-sm);
	}
	.preview-banner h3 {
		margin: 0;
		font-size: var(--fs-md);
	}
	.preview-banner p {
		margin-block-start: var(--space-2);
		margin-block-end: var(--space-3);
		font-size: var(--fs-sm);
		color: var(--surface-fg-muted);
	}
	.preview-actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.preview-btn {
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		border: 1px solid var(--surface-border-subtle);
		font-size: var(--fs-xs);
		cursor: pointer;
	}
	.preview-btn--primary {
		background-color: var(--brand-teal-500);
		color: white;
		border-color: var(--brand-teal-500);
	}
	.preview-btn--secondary {
		background-color: transparent;
	}
	.preview-btn--tertiary {
		background-color: transparent;
		border-color: transparent;
	}
</style>
