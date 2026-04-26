<!--
  FORM-09: Create form.

  Minimal intake (name, slug, description); schema + logic are empty on
  create and edited in the builder after redirect. Slug autofills from
  the name but stays editable; validation uses native HTML constraints.

  A11y: required + pattern are declared on inputs; the API error is
  announced via `role="alert"` + `aria-live="polite"` so SR users hear
  server-side failures.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';

	let name = $state('');
	let slug = $state('');
	let description = $state('');
	let saving = $state(false);
	let err = $state<string | null>(null);

	const slugFromName = $derived(
		name
			.toLowerCase()
			.replace(/[^a-z0-9\s-]/g, '')
			.trim()
			.replace(/\s+/g, '-')
			.slice(0, 64)
	);

	let slugTouched = $state(false);

	$effect(() => {
		if (!slugTouched) slug = slugFromName;
	});

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		err = null;
		saving = true;
		try {
			const created = await api.post<{ id: string }>('/admin/forms', {
				name,
				slug,
				description: description || null,
				schema: [],
				logic: []
			});
			await goto(resolve('/admin/forms/[id]', { id: created.id }));
		} catch (e2) {
			err = e2 instanceof Error ? e2.message : 'Failed to create form.';
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head><title>New form · Admin</title></svelte:head>

<header class="nf-header">
	<a class="nf-back" href={resolve('/admin/forms')}>← Forms</a>
	<h1 class="nf-title">New form</h1>
</header>

<form class="nf-form" onsubmit={submit}>
	<label class="nf-field">
		<span class="nf-label">Name</span>
		<input
			class="nf-input"
			type="text"
			bind:value={name}
			required
			minlength="1"
			maxlength="120"
			autocomplete="off"
		/>
	</label>

	<label class="nf-field">
		<span class="nf-label">Slug</span>
		<input
			class="nf-input nf-input--mono"
			type="text"
			bind:value={slug}
			required
			pattern="[a-z0-9-]+"
			oninput={() => (slugTouched = true)}
		/>
		<small class="nf-hint">URL path: /forms/{slug || 'your-slug'}</small>
	</label>

	<label class="nf-field">
		<span class="nf-label">Description</span>
		<textarea class="nf-input nf-input--multi" rows="3" bind:value={description} maxlength="500"
		></textarea>
	</label>

	{#if err}
		<p class="nf-error" role="alert" aria-live="polite">{err}</p>
	{/if}

	<div class="nf-actions">
		<button
			class="nf-btn nf-btn--ghost"
			type="button"
			onclick={() => goto(resolve('/admin/forms'))}
		>
			Cancel
		</button>
		<button class="nf-btn nf-btn--primary" type="submit" disabled={saving}>
			{saving ? 'Creating…' : 'Create form'}
		</button>
	</div>
</form>

<style>
	.nf-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-block-end: var(--space-4);
	}

	.nf-back {
		color: var(--surface-fg-muted);
		font-size: var(--fs-sm);
		text-decoration: none;
	}

	.nf-back:hover {
		color: var(--surface-fg-default);
	}

	.nf-title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		color: var(--surface-fg-default);
	}

	.nf-form {
		display: grid;
		gap: var(--space-4);
		max-inline-size: 36rem;
	}

	.nf-field {
		display: grid;
		gap: var(--space-1-5);
	}

	.nf-label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--surface-fg-default);
	}

	.nf-input {
		padding-block: var(--space-2-5);
		padding-inline: var(--space-3);
		font-size: var(--fs-md);
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		border: 1px solid var(--surface-border-default);
		border-radius: var(--radius-md);
	}

	.nf-input:focus-visible {
		outline: none;
		border-color: var(--brand-teal-500);
		box-shadow: 0 0 0 3px oklch(0.66 0.12 197 / 0.25);
	}

	.nf-input--mono {
		font-family: var(--font-mono);
	}

	.nf-input--multi {
		resize: vertical;
		min-block-size: 6rem;
	}

	.nf-hint {
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
	}

	.nf-error {
		padding: var(--space-3);
		background-color: var(--status-danger-50);
		color: var(--status-danger-700);
		border-radius: var(--radius-md);
		border: 1px solid var(--status-danger-500);
		font-size: var(--fs-sm);
	}

	.nf-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}

	.nf-btn {
		padding-block: var(--space-2);
		padding-inline: var(--space-4);
		border-radius: var(--radius-md);
		font-family: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		border: 1px solid transparent;
	}

	.nf-btn--ghost {
		background-color: transparent;
		color: var(--surface-fg-default);
		border-color: var(--surface-border-default);
	}

	.nf-btn--ghost:hover {
		background-color: var(--surface-bg-muted);
	}

	.nf-btn--primary {
		background-color: var(--brand-teal-500);
		color: var(--neutral-0);
	}

	.nf-btn--primary:hover:not(:disabled) {
		background-color: var(--brand-teal-600);
	}

	.nf-btn:focus-visible {
		outline: 2px solid var(--brand-teal-500);
		outline-offset: 2px;
	}

	.nf-btn:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}
</style>
