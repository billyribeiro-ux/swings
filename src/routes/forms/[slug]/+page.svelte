<!--
  FORM-10: Public form page.

  Renders the FormRenderer with the SSR-loaded definition. The renderer
  owns every concern the plan calls for: validation, submission, logic,
  multi-step, save-and-resume, honeypot.

  Layout is intentionally minimal — a centred `<main>` with the form's
  name + description above the renderer. Sites that embed a form on
  arbitrary marketing pages should use the FormRenderer component
  directly; this route is the canonical standalone URL.
-->
<script lang="ts">
	import type { PageData } from './$types';
	import FormRenderer from '$lib/components/forms/FormRenderer.svelte';
	import type { FieldSchema, LogicRule, FormSettings } from '$lib/components/forms/types';
	import { toasts } from '$lib/stores/toasts.svelte';

	const { data }: { data: PageData } = $props();

	// The OpenAPI-generated `FormDefinition.schema` / `.logic` / `.settings` are
	// typed as `unknown` because the backend uses inline enums — so we narrow
	// at the boundary. The server is trusted; we never mutate these.
	const schema = $derived<readonly FieldSchema[]>(
		Array.isArray(data.definition.schema) ? (data.definition.schema as FieldSchema[]) : []
	);
	const logic = $derived<readonly LogicRule[]>(
		Array.isArray(data.definition.logic) ? (data.definition.logic as LogicRule[]) : []
	);
	const settings = $derived<FormSettings>(
		data.definition.settings && typeof data.definition.settings === 'object'
			? (data.definition.settings as FormSettings)
			: {}
	);

	function handleSuccess(submissionId: string) {
		toasts.push({
			kind: 'success',
			title: 'Submitted',
			description: submissionId
				? `Your reference is ${submissionId.slice(0, 8)}.`
				: 'Your response has been recorded.'
		});
	}

	function handleError(message: string) {
		toasts.push({ kind: 'danger', title: 'Submission failed', description: message });
	}
</script>

<svelte:head>
	<title>{data.definition.name}</title>
	{#if data.definition.description}
		<meta name="description" content={data.definition.description} />
	{/if}
</svelte:head>

<main class="public-form">
	<header class="public-form__header">
		<h1 class="public-form__title">{data.definition.name}</h1>
		{#if data.definition.description}
			<p class="public-form__description">{data.definition.description}</p>
		{/if}
	</header>

	<FormRenderer
		slug={data.definition.slug}
		{schema}
		{logic}
		{settings}
		onSuccess={handleSuccess}
		onError={handleError}
	/>
</main>

<style>
	.public-form {
		max-inline-size: 48rem;
		margin-inline: auto;
		padding-block: var(--space-8);
		padding-inline: var(--space-5);
	}

	.public-form__header {
		margin-block-end: var(--space-6);
	}

	.public-form__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-semibold);
		color: var(--surface-fg-default);
	}

	.public-form__description {
		margin-block-start: var(--space-2);
		color: var(--surface-fg-muted);
		font-size: var(--fs-md);
		line-height: var(--lh-relaxed);
	}

	@media (min-width: 768px) {
		.public-form {
			padding-block: var(--space-12);
		}
	}
</style>
