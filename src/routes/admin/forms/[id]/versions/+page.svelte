<!--
  FORM-09: Version diff viewer.

  Lists every saved version for a form (schema + logic snapshots) and
  surfaces a side-by-side structural diff between the selected pair.
  The diff operates on field keys as the stable identifier:
  - added   — keys present only in v2
  - removed — keys present only in v1
  - changed — keys in both but with divergent JSON payloads (deep-equal)

  Decorative-only fields (`html_block`, `page_break`, etc.) still show up
  because admin edits them too; we don't filter on render because the
  keys are the documented stable identifier per backend/src/forms/schema.rs.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api } from '$lib/api/client';

	interface Version {
		readonly id: string;
		readonly version: number;
		readonly schema: readonly FieldRow[];
		readonly logic: readonly unknown[];
		readonly created_at: string;
		readonly created_by: string;
	}

	interface FieldRow {
		readonly type: string;
		readonly key: string;
		readonly label?: string;
		readonly [extra: string]: unknown;
	}

	const id = $derived(page.params.id ?? '');

	let versions = $state<Version[]>([]);
	let loading = $state(true);
	let err = $state<string | null>(null);
	let leftId = $state<string | null>(null);
	let rightId = $state<string | null>(null);

	onMount(async () => {
		try {
			versions = await api.get<Version[]>(`/admin/forms/${id}/versions`);
			if (versions.length > 0) {
				rightId = versions[0]!.id;
				leftId = versions[1]?.id ?? versions[0]!.id;
			}
		} catch (e) {
			err = e instanceof Error ? e.message : 'Failed to load versions.';
		} finally {
			loading = false;
		}
	});

	const left = $derived(versions.find((v) => v.id === leftId) ?? null);
	const right = $derived(versions.find((v) => v.id === rightId) ?? null);

	interface Diff {
		readonly added: FieldRow[];
		readonly removed: FieldRow[];
		readonly changed: Array<{ key: string; from: FieldRow; to: FieldRow }>;
		readonly unchanged: number;
	}

	const diff = $derived<Diff>(computeDiff(left, right));

	function computeDiff(a: Version | null, b: Version | null): Diff {
		if (!a || !b) return { added: [], removed: [], changed: [], unchanged: 0 };
		const am = new Map(a.schema.map((f) => [f.key, f]));
		const bm = new Map(b.schema.map((f) => [f.key, f]));
		const added: FieldRow[] = [];
		const removed: FieldRow[] = [];
		const changed: Array<{ key: string; from: FieldRow; to: FieldRow }> = [];
		let unchanged = 0;
		for (const [key, to] of bm) {
			const from = am.get(key);
			if (!from) {
				added.push(to);
			} else if (!deepEqual(from, to)) {
				changed.push({ key, from, to });
			} else {
				unchanged += 1;
			}
		}
		for (const [key, from] of am) {
			if (!bm.has(key)) removed.push(from);
		}
		return { added, removed, changed, unchanged };
	}

	function deepEqual(a: unknown, b: unknown): boolean {
		if (a === b) return true;
		if (a === null || b === null || typeof a !== typeof b) return false;
		if (Array.isArray(a) && Array.isArray(b)) {
			if (a.length !== b.length) return false;
			return a.every((el, i) => deepEqual(el, b[i]));
		}
		if (typeof a === 'object' && typeof b === 'object') {
			const ao = a as Record<string, unknown>;
			const bo = b as Record<string, unknown>;
			const keys = new Set([...Object.keys(ao), ...Object.keys(bo)]);
			for (const k of keys) {
				if (!deepEqual(ao[k], bo[k])) return false;
			}
			return true;
		}
		return false;
	}

	function formatField(f: FieldRow): string {
		const label = f.label ? ` · "${f.label}"` : '';
		return `${f.type} ${f.key}${label}`;
	}

	function formatDate(s: string): string {
		return new Date(s).toLocaleString();
	}
</script>

<svelte:head><title>Versions · Form Builder</title></svelte:head>

<header class="versions__header">
	<a class="versions__back" href={`/admin/forms/${id}`}>← Builder</a>
	<h1 class="versions__title">Versions</h1>
</header>

{#if loading}
	<p class="versions__loading">Loading…</p>
{:else if err}
	<p class="versions__error" role="alert">{err}</p>
{:else if versions.length === 0}
	<p class="versions__empty">No saved versions yet.</p>
{:else}
	<div class="versions__picker">
		<label class="versions__field">
			<span>Compare from</span>
			<select bind:value={leftId}>
				{#each versions as v (v.id)}
					<option value={v.id}>v{v.version} — {formatDate(v.created_at)}</option>
				{/each}
			</select>
		</label>
		<label class="versions__field">
			<span>Compare to</span>
			<select bind:value={rightId}>
				{#each versions as v (v.id)}
					<option value={v.id}>v{v.version} — {formatDate(v.created_at)}</option>
				{/each}
			</select>
		</label>
	</div>

	<section class="versions__summary" aria-live="polite">
		<span class="versions__pill versions__pill--added">+{diff.added.length} added</span>
		<span class="versions__pill versions__pill--removed">−{diff.removed.length} removed</span>
		<span class="versions__pill versions__pill--changed">~{diff.changed.length} changed</span>
		<span class="versions__pill versions__pill--unchanged">{diff.unchanged} unchanged</span>
	</section>

	<div class="versions__grid">
		<article class="versions__col">
			<h2>Added</h2>
			{#if diff.added.length === 0}
				<p class="versions__none">None</p>
			{:else}
				<ul class="versions__list">
					{#each diff.added as f (f.key)}
						<li class="versions__item versions__item--added">{formatField(f)}</li>
					{/each}
				</ul>
			{/if}
		</article>

		<article class="versions__col">
			<h2>Removed</h2>
			{#if diff.removed.length === 0}
				<p class="versions__none">None</p>
			{:else}
				<ul class="versions__list">
					{#each diff.removed as f (f.key)}
						<li class="versions__item versions__item--removed">{formatField(f)}</li>
					{/each}
				</ul>
			{/if}
		</article>

		<article class="versions__col">
			<h2>Changed</h2>
			{#if diff.changed.length === 0}
				<p class="versions__none">None</p>
			{:else}
				<ul class="versions__list">
					{#each diff.changed as c (c.key)}
						<li class="versions__item versions__item--changed">
							<details>
								<summary>{c.key} · {c.to.type}</summary>
								<pre class="versions__pre">
from: {JSON.stringify(c.from, null, 2)}
to:   {JSON.stringify(c.to, null, 2)}
								</pre>
							</details>
						</li>
					{/each}
				</ul>
			{/if}
		</article>
	</div>
{/if}

<style>
	.versions__header {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-block-end: var(--space-4);
	}

	.versions__back {
		color: var(--surface-fg-muted);
		font-size: var(--fs-sm);
		text-decoration: none;
	}

	.versions__back:hover {
		color: var(--surface-fg-default);
	}

	.versions__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
	}

	.versions__loading,
	.versions__empty {
		padding: var(--space-8);
		text-align: center;
		color: var(--surface-fg-muted);
	}

	.versions__error {
		padding: var(--space-4);
		color: var(--status-danger-700);
	}

	.versions__picker {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
		margin-block-end: var(--space-4);
	}

	.versions__field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-inline-size: 18rem;
	}

	.versions__field > span {
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.versions__field select {
		padding-block: var(--space-2);
		padding-inline: var(--space-3);
		border: 1px solid var(--surface-border-default);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
		font-size: var(--fs-sm);
		color: var(--surface-fg-default);
	}

	.versions__field select:focus-visible {
		outline: 2px solid var(--brand-teal-500);
		outline-offset: 2px;
	}

	.versions__summary {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-block-end: var(--space-4);
	}

	.versions__pill {
		display: inline-flex;
		padding-block: var(--space-1);
		padding-inline: var(--space-2-5);
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		font-variant-numeric: tabular-nums;
	}

	.versions__pill--added {
		background-color: var(--status-success-50);
		color: var(--status-success-700);
	}

	.versions__pill--removed {
		background-color: var(--status-danger-50);
		color: var(--status-danger-700);
	}

	.versions__pill--changed {
		background-color: var(--status-warning-50);
		color: var(--status-warning-700);
	}

	.versions__pill--unchanged {
		background-color: var(--surface-bg-muted);
		color: var(--surface-fg-muted);
	}

	.versions__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: var(--space-4);
	}

	@media (min-width: 1024px) {
		.versions__grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.versions__col h2 {
		margin: 0 0 var(--space-2);
		font-size: var(--fs-md);
		color: var(--surface-fg-default);
	}

	.versions__list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.versions__item {
		padding: var(--space-2);
		font-size: var(--fs-sm);
		border-radius: var(--radius-sm);
		font-family: var(--font-mono);
	}

	.versions__item--added {
		background-color: var(--status-success-50);
		color: var(--status-success-700);
	}

	.versions__item--removed {
		background-color: var(--status-danger-50);
		color: var(--status-danger-700);
	}

	.versions__item--changed {
		background-color: var(--status-warning-50);
		color: var(--status-warning-700);
	}

	.versions__item--changed details summary {
		cursor: pointer;
	}

	.versions__pre {
		margin: var(--space-2) 0 0;
		font-size: var(--fs-xs);
		white-space: pre-wrap;
		overflow-x: auto;
	}

	.versions__none {
		margin: 0;
		padding: var(--space-3);
		color: var(--surface-fg-muted);
		font-size: var(--fs-sm);
		text-align: center;
		background-color: var(--surface-bg-subtle);
		border-radius: var(--radius-md);
	}
</style>
