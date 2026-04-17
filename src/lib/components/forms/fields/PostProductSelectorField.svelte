<!--
  FORM-10: Picker that resolves to a blog post or product id. Searchable
  combobox: typed input filters a results list fetched from
  `/api/admin/posts?search=` or `/api/admin/products?search=` depending
  on the configured source.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import { api } from '$lib/api/client';
	import type { FieldProps, FieldSchema } from '../types.ts';

	interface Candidate {
		id: string;
		title: string;
	}

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const p = $derived(field as Extract<FieldSchema, { type: 'post_product_selector' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);
	const listboxId = $derived(`${controlId}-list`);

	let query = $state('');
	let results = $state<Candidate[]>([]);
	let open = $state(false);
	let searching = $state(false);

	const endpoint = $derived(p.source === 'product' ? '/api/products' : '/api/posts');

	async function runSearch() {
		searching = true;
		try {
			const res = await api.get<Candidate[]>(
				`${endpoint}?search=${encodeURIComponent(query)}&per_page=10`,
				{ skipAuth: true }
			);
			results = res;
		} catch {
			results = [];
		} finally {
			searching = false;
		}
	}

	let debounceHandle: ReturnType<typeof setTimeout> | null = null;
	$effect(() => {
		const q = query;
		if (debounceHandle) clearTimeout(debounceHandle);
		debounceHandle = setTimeout(() => {
			if (q.length === 0) {
				results = [];
			} else {
				void runSearch();
			}
		}, 200);
		return () => {
			if (debounceHandle) clearTimeout(debounceHandle);
		};
	});

	function pick(c: Candidate) {
		onChange(field.key, c.id);
		query = c.title;
		open = false;
	}
</script>

<FieldFrame
	{controlId}
	label={field.label ?? field.key}
	helpText={field.helpText}
	{error}
	required={field.required ?? false}
>
	{#snippet children({ describedBy, invalid, required })}
		<div class="fm-combo">
			<input
				id={controlId}
				name={field.key}
				type="text"
				class="fm-input"
				role="combobox"
				value={query.length > 0 ? query : current}
				{disabled}
				autocomplete="off"
				aria-controls={listboxId}
				aria-expanded={open}
				aria-autocomplete="list"
				aria-describedby={describedBy}
				aria-invalid={invalid}
				aria-required={required}
				placeholder={field.placeholder ?? 'Start typing to search…'}
				oninput={(e) => {
					query = (e.currentTarget as HTMLInputElement).value;
					open = true;
				}}
				onfocus={() => (open = query.length > 0)}
			/>
			{#if open && (results.length > 0 || searching)}
				<ul id={listboxId} role="listbox" class="fm-combo__list">
					{#if searching}
						<li class="fm-combo__empty" aria-live="polite">Searching…</li>
					{:else}
						{#each results as r (r.id)}
							<li role="option" aria-selected={current === r.id}>
								<button
									type="button"
									class="fm-combo__option"
									onclick={() => pick(r)}
								>
									{r.title}
								</button>
							</li>
						{/each}
					{/if}
				</ul>
			{/if}
		</div>
	{/snippet}
</FieldFrame>
