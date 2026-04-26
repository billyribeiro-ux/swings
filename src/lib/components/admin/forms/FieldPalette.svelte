<script lang="ts">
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';

	type Props = { onAdd: (type: string) => void };
	const { onAdd }: Props = $props();

	// FORM-09: 33 field types in the order the renderer dispatches on.
	// Each tuple is `[type, label, group]`; the palette is grouped so
	// long lists scroll less.
	const fields: Array<[string, string, string]> = [
		['text', 'Text', 'Basic'],
		['email', 'Email', 'Basic'],
		['phone', 'Phone', 'Basic'],
		['url', 'URL', 'Basic'],
		['textarea', 'Textarea', 'Basic'],
		['number', 'Number', 'Basic'],
		['hidden', 'Hidden', 'Basic'],
		['select', 'Select', 'Choice'],
		['multi_select', 'Multi-select', 'Choice'],
		['radio', 'Radio', 'Choice'],
		['checkbox', 'Checkbox', 'Choice'],
		['date', 'Date', 'Date / Time'],
		['time', 'Time', 'Date / Time'],
		['datetime', 'Datetime', 'Date / Time'],
		['slider', 'Slider', 'Numeric'],
		['rating', 'Rating', 'Numeric'],
		['file_upload', 'File upload', 'Upload'],
		['image_upload', 'Image upload', 'Upload'],
		['signature', 'Signature', 'Upload'],
		['rich_text', 'Rich text', 'Content'],
		['html_block', 'HTML block', 'Content'],
		['custom_html', 'Custom HTML', 'Content'],
		['section_break', 'Section break', 'Layout'],
		['page_break', 'Page break', 'Layout'],
		['address', 'Address', 'Specialised'],
		['gdpr_consent', 'GDPR consent', 'Specialised'],
		['terms', 'Terms', 'Specialised'],
		['country_state', 'Country / state', 'Specialised'],
		['post_product_selector', 'Post / product picker', 'Specialised'],
		['payment', 'Payment', 'Payment'],
		['subscription', 'Subscription', 'Payment'],
		['quiz', 'Quiz', 'Survey'],
		['nps', 'NPS', 'Survey'],
		['likert', 'Likert', 'Survey'],
		['matrix', 'Matrix', 'Survey'],
		['ranking', 'Ranking', 'Survey'],
		['calculation', 'Calculation', 'Compute'],
		['dynamic_dropdown', 'Dynamic dropdown', 'Compute']
	];

	const groups = $derived(
		Array.from(new Set(fields.map((f) => f[2]))).map((g) => ({
			name: g,
			items: fields.filter((f) => f[2] === g)
		}))
	);
</script>

<aside class="palette" aria-label="Field palette">
	<h2 class="palette__title">Fields</h2>
	{#each groups as group (group.name)}
		<section class="palette__group">
			<h3 class="palette__group-title">{group.name}</h3>
			<ul class="palette__list">
				{#each group.items as [type, label] (type)}
					<li>
						<button
							type="button"
							class="palette__btn"
							onclick={() => onAdd(type)}
							draggable="true"
							ondragstart={(e) => e.dataTransfer?.setData('text/x-form-field', type)}
						>
							<PlusIcon size={14} />
							{label}
						</button>
					</li>
				{/each}
			</ul>
		</section>
	{/each}
</aside>

<style>
	.palette {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-3);
		border-inline-end: 1px solid var(--color-border);
		overflow-y: auto;
		min-block-size: 100%;
	}
	.palette__title {
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin-block-end: var(--space-1);
	}
	.palette__group-title {
		font-size: var(--font-size-2xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-faint);
		margin-block: var(--space-2) var(--space-1);
	}
	.palette__list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		gap: var(--space-1);
	}
	.palette__btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		inline-size: 100%;
		padding: var(--space-1) var(--space-2);
		font-size: var(--font-size-sm);
		text-align: start;
		background: var(--color-surface-2);
		color: var(--color-text);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		cursor: grab;
		transition: background 120ms ease;
	}
	.palette__btn:hover {
		background: var(--color-surface-3);
	}
	.palette__btn:focus-visible {
		outline: 2px solid var(--color-focus-ring);
		outline-offset: 2px;
	}
</style>
