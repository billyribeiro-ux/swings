<script lang="ts">
	import { goto } from '$app/navigation';
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

	$effect(() => {
		if (!slug) slug = slugFromName;
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
			await goto(`/admin/forms/${created.id}`);
		} catch (e2) {
			err = e2 instanceof Error ? e2.message : 'Failed to create form.';
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head><title>New form · Admin</title></svelte:head>

<header class="page-header">
	<a href="/admin/forms" class="back">← Forms</a>
	<h1>New form</h1>
</header>

<form class="form" onsubmit={submit}>
	<label class="field">
		<span class="field__label">Name</span>
		<input
			class="field__input"
			type="text"
			bind:value={name}
			required
			minlength="1"
			maxlength="120"
		/>
	</label>
	<label class="field">
		<span class="field__label">Slug</span>
		<input
			class="field__input field__input--mono"
			type="text"
			bind:value={slug}
			required
			pattern="[a-z0-9-]+"
		/>
		<small class="field__hint">URL path: /forms/{slug || 'your-slug'}</small>
	</label>
	<label class="field">
		<span class="field__label">Description</span>
		<textarea
			class="field__input field__input--multi"
			rows="3"
			bind:value={description}
		></textarea>
	</label>

	{#if err}<p class="error">{err}</p>{/if}

	<div class="actions">
		<button class="btn" type="button" onclick={() => goto('/admin/forms')}>Cancel</button>
		<button class="btn btn--primary" type="submit" disabled={saving}>
			{saving ? 'Creating…' : 'Create form'}
		</button>
	</div>
</form>

<style>
	.page-header { margin-block-end: var(--space-4); }
	.back { color: var(--color-text-muted); font-size: var(--font-size-sm); text-decoration: none; }
	.back:hover { color: var(--color-text); }
	.form { display: grid; gap: var(--space-3); max-inline-size: 60ch; }
	.field { display: grid; gap: var(--space-1); }
	.field__label {
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.field__input {
		padding: var(--space-2);
		font-size: var(--font-size-sm);
		background: var(--color-surface-2);
		color: var(--color-text);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
	}
	.field__input:focus-visible {
		outline: 2px solid var(--color-focus-ring);
		outline-offset: 1px;
	}
	.field__input--mono { font-family: var(--font-mono); }
	.field__input--multi { resize: vertical; min-block-size: 6lh; }
	.field__hint { font-size: var(--font-size-2xs); color: var(--color-text-muted); }
	.actions { display: flex; gap: var(--space-2); }
	.btn {
		padding: var(--space-1) var(--space-3);
		border-radius: var(--radius-sm);
		font-size: var(--font-size-sm);
		cursor: pointer;
		border: 1px solid var(--color-border);
		background: var(--color-surface-2);
		color: var(--color-text);
	}
	.btn--primary { background: var(--color-accent); color: var(--color-on-accent); border-color: transparent; }
	.btn:disabled { opacity: 0.6; cursor: wait; }
	.error { color: var(--color-danger); font-size: var(--font-size-sm); }
</style>
