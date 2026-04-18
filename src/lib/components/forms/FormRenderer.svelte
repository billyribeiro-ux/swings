<!--
  FORM-10: Public renderer.

  Takes a `FormDefinition` (`schema`, `logic`, `settings`) as props and drives
  the full lifecycle:
  - Hydrates a data map.
  - Runs the shared validator (`$lib/forms/validate`) on submit; the same
    errors land server-side so messaging is symmetric.
  - POSTs to `/api/forms/{slug}/submit` via the `submitForm` helper.
  - Fires an `onSuccess(submissionId)` callback upon success.

  Honeypot: `form_hp` is always emitted (FORM-06). Logic: `show` / `hide`
  rules from `logic` (server-mirrored) gate visibility. Multi-step support is
  included via PageBreak handling; for single-step forms every field is in
  step 0.

  WCAG 2.2 AA: an `aria-live="polite"` error summary at the top surfaces
  validation failures; `:focus-visible` rings live in `forms.css`.
-->
<script lang="ts" module>
	import type {
		FieldSchema,
		LogicRule,
		FormSettings,
		LogicCondition,
		LogicAction
	} from './types.ts';

	export interface FormRendererProps {
		/** Stable identifier used in POST paths + localStorage resume keys. */
		readonly slug: string;
		readonly schema: readonly FieldSchema[];
		readonly logic?: readonly LogicRule[];
		readonly settings?: FormSettings;
		/** Called with the submission UUID once the server accepts the payload. */
		readonly onSuccess?: (submissionId: string) => void;
		/** Called on non-2xx responses. */
		readonly onError?: (message: string) => void;
	}
</script>

<script lang="ts">
	import './forms.css';
	import { onMount, tick } from 'svelte';
	import {
		loadPartial,
		savePartial,
		submitForm
	} from '$lib/api/forms';
	import { validate } from '$lib/forms/validate';
	import type { ValidationError, FormDataMap } from './types.ts';
	import FormField from './FormField.svelte';
	import StepProgress from './StepProgress.svelte';

	const {
		slug,
		schema,
		logic = [],
		settings,
		onSuccess,
		onError
	}: FormRendererProps = $props();

	// ── Reactive state ───────────────────────────────────────────────────
	let data = $state<Record<string, unknown>>({});
	let errors = $state<ValidationError[]>([]);
	let submitting = $state(false);
	let submitted = $state(false);
	let honeypot = $state('');
	let currentStep = $state(0);
	let resumeToken = $state<string | null>(null);
	let summaryEl: HTMLDivElement | null = $state(null);

	// ── Logic evaluator (mirrors backend/src/forms/logic.rs) ─────────────
	function evaluateCondition(cond: LogicCondition, d: FormDataMap): boolean {
		switch (cond.op) {
			case 'field_equals':
				return cond.field !== undefined && d[cond.field] === cond.value;
			case 'field_not_equals':
				return cond.field !== undefined && d[cond.field] !== cond.value;
			case 'field_greater_than': {
				const v = cond.field !== undefined ? d[cond.field] : undefined;
				const n = typeof v === 'number' ? v : typeof v === 'string' ? Number(v) : NaN;
				const threshold = typeof cond.value === 'number' ? cond.value : NaN;
				return Number.isFinite(n) && Number.isFinite(threshold) && n > threshold;
			}
			case 'field_less_than': {
				const v = cond.field !== undefined ? d[cond.field] : undefined;
				const n = typeof v === 'number' ? v : typeof v === 'string' ? Number(v) : NaN;
				const threshold = typeof cond.value === 'number' ? cond.value : NaN;
				return Number.isFinite(n) && Number.isFinite(threshold) && n < threshold;
			}
			case 'field_contains': {
				const v = cond.field !== undefined ? d[cond.field] : undefined;
				const needle = typeof cond.value === 'string' ? cond.value : '';
				if (typeof v === 'string') return v.includes(needle);
				if (Array.isArray(v)) return v.some((e) => typeof e === 'string' && e === needle);
				return false;
			}
			case 'and':
				return (cond.conditions ?? []).every((c) => evaluateCondition(c, d));
			case 'or':
				return (cond.conditions ?? []).length > 0 && (cond.conditions ?? []).some((c) => evaluateCondition(c, d));
			default:
				return false;
		}
	}

	/** Returns the per-rule verdicts for the current data map. */
	const verdict = $derived.by(() => {
		const hidden = new Set<string>();
		const shown = new Set<string>();
		const required = new Set<string>();
		const skippedSteps = new Set<number>();
		const setValues: Array<{ field: string; value: unknown }> = [];
		for (const rule of logic) {
			if (!evaluateCondition(rule.condition, data)) continue;
			const a: LogicAction = rule.action;
			if (a.kind === 'hide' && a.field) hidden.add(a.field);
			else if (a.kind === 'show' && a.field) shown.add(a.field);
			else if (a.kind === 'require_field' && a.field) required.add(a.field);
			else if (a.kind === 'skip_step' && typeof a.step === 'number') skippedSteps.add(a.step);
			else if (a.kind === 'set_value' && a.field) setValues.push({ field: a.field, value: a.value });
		}
		return { hidden, shown, required, skippedSteps, setValues };
	});

	$effect(() => {
		// Apply `set_value` deterministically — but only if the current value differs,
		// so we don't feedback-loop.
		for (const sv of verdict.setValues) {
			if (data[sv.field] !== sv.value) {
				data[sv.field] = sv.value;
			}
		}
	});

	function isFieldVisible(field: FieldSchema): boolean {
		if (verdict.hidden.has(field.key)) return false;
		return true;
	}

	function isFieldRequired(field: FieldSchema): boolean {
		if (verdict.required.has(field.key)) return true;
		return field.required === true;
	}

	// ── Steps (PageBreak boundaries) ─────────────────────────────────────
	const steps = $derived.by(() => {
		const out: FieldSchema[][] = [[]];
		for (const f of schema) {
			if (f.type === 'page_break') {
				out.push([]);
			} else {
				out[out.length - 1].push(f);
			}
		}
		return out;
	});

	/** Page break labels become step names. First step falls back to 'Start'. */
	const stepLabels = $derived.by(() => {
		const out: string[] = ['Start'];
		for (const f of schema) {
			if (f.type === 'page_break') {
				out.push(f.label && f.label.length > 0 ? f.label : `Step ${out.length + 1}`);
			}
		}
		return out;
	});

	const totalSteps = $derived(steps.length);
	const isMultiStep = $derived(totalSteps > 1);
	const currentFields = $derived(steps[Math.min(currentStep, steps.length - 1)] ?? []);

	// ── Field writer ─────────────────────────────────────────────────────
	function setField(key: string, value: unknown): void {
		data[key] = value;
		// Clear only this field's existing errors on edit; the summary regenerates on submit.
		if (errors.length > 0) {
			errors = errors.filter((e) => e.field_key !== key);
		}
	}

	// ── Save + resume ────────────────────────────────────────────────────
	const storageKey = $derived(`forms:resume:${slug}`);

	async function persistPartial() {
		if (!settings?.save_and_resume) return;
		try {
			const res = await savePartial(slug, data, resumeToken ?? undefined);
			resumeToken = res.resume_token;
			try {
				localStorage.setItem(storageKey, res.resume_token);
			} catch {
				// ignore quota / private mode
			}
		} catch {
			// surface via toast? silent by design — partial persistence is best-effort.
		}
	}

	async function hydrateFromResume() {
		if (!settings?.save_and_resume) return;
		try {
			const url = new URL(window.location.href);
			const param = url.searchParams.get('resume');
			let token = param ?? null;
			if (!token) {
				try {
					token = localStorage.getItem(storageKey);
				} catch {
					token = null;
				}
			}
			if (!token) return;
			const snapshot = await loadPartial(slug, token);
			resumeToken = token;
			for (const [k, v] of Object.entries(snapshot ?? {})) {
				data[k] = v;
			}
		} catch {
			// ignore — invalid / expired token leaves state empty.
		}
	}

	// ── Step navigation ──────────────────────────────────────────────────
	async function goNext() {
		// Validate only the current step's fields so the user isn't blocked by later pages.
		const stepErrors = await validate(currentFields, data);
		if (stepErrors.length > 0) {
			errors = stepErrors;
			await tick();
			summaryEl?.focus();
			return;
		}
		errors = [];
		await persistPartial();
		// Skip hidden / skipped steps forward.
		let next = currentStep + 1;
		while (next < totalSteps && verdict.skippedSteps.has(next)) next += 1;
		currentStep = Math.min(next, totalSteps - 1);
	}

	function goPrev() {
		errors = [];
		let prev = currentStep - 1;
		while (prev >= 0 && verdict.skippedSteps.has(prev)) prev -= 1;
		currentStep = Math.max(prev, 0);
	}

	// ── Submit ───────────────────────────────────────────────────────────
	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (submitting) return;

		// Honeypot: FORM-06 always rejects a non-empty hp field.
		if (honeypot.length > 0) {
			onSuccess?.('honeypot-skipped');
			return;
		}

		errors = [];
		submitting = true;
		try {
			const allFieldsVisible = schema.filter(isFieldVisible);
			const vErrors = await validate(allFieldsVisible, data);
			if (vErrors.length > 0) {
				errors = vErrors;
				await tick();
				summaryEl?.focus();
				return;
			}
			const res = await submitForm(slug, data);
			submitted = true;
			// Clean up the resume token on success.
			try {
				localStorage.removeItem(storageKey);
			} catch {
				// ignore
			}
			// Response shape per backend/src/handlers/forms.rs:SubmitResponse = { id, status }.
			const submissionId =
				res && typeof res === 'object' && 'id' in res && typeof res.id === 'string'
					? res.id
					: '';
			onSuccess?.(submissionId);
		} catch (err) {
			const msg = err instanceof Error ? err.message : 'Submission failed. Please try again.';
			onError?.(msg);
		} finally {
			submitting = false;
		}
	}

	onMount(() => {
		void hydrateFromResume();
	});
</script>

<form class="fm-form" novalidate onsubmit={handleSubmit} aria-labelledby="fm-form-title">
	{#if isMultiStep}
		<StepProgress current={currentStep} total={totalSteps} labels={stepLabels} />
	{/if}

	{#if errors.length > 0}
		<div
			bind:this={summaryEl}
			class="fm-error-summary"
			role="alert"
			aria-live="polite"
			tabindex="-1"
		>
			<p class="fm-error-summary__title">
				There {errors.length === 1 ? 'is 1 problem' : `are ${errors.length} problems`} with your submission.
			</p>
			<ul class="fm-error-summary__list">
				{#each errors as e (e.field_key + e.code)}
					<li><a href={`#form-field-${e.field_key}`}>{e.message}</a></li>
				{/each}
			</ul>
		</div>
	{/if}

	{#if submitted}
		<div class="fm-success" role="status" aria-live="polite">
			{settings?.success_message ?? 'Thanks — your submission has been received.'}
		</div>
	{:else}
		<div class="fm-fields">
			{#each currentFields as field (field.key + field.type)}
				{#if isFieldVisible(field)}
					<FormField
						field={{ ...field, required: isFieldRequired(field) }}
						value={data[field.key]}
						data={data}
						error={errors.find((e) => e.field_key === field.key)?.message}
						disabled={submitting}
						onChange={setField}
					/>
				{/if}
			{/each}
		</div>

		<!--
		  Honeypot: offscreen, aria-hidden, tabindex -1 so humans never reach it
		  but bots commonly fill every field. Rejected server-side (FORM-06).
		-->
		<div class="fm-honeypot" aria-hidden="true">
			<label for="fm-form-hp">Leave this field empty</label>
			<input
				id="fm-form-hp"
				name="form_hp"
				type="text"
				autocomplete="off"
				tabindex={-1}
				bind:value={honeypot}
			/>
		</div>

		<div class="fm-actions">
			{#if isMultiStep && currentStep > 0}
				<button type="button" class="fm-btn fm-btn--ghost" onclick={goPrev} disabled={submitting}>
					Back
				</button>
			{/if}
			{#if isMultiStep && currentStep < totalSteps - 1}
				<button type="button" class="fm-btn fm-btn--primary" onclick={goNext} disabled={submitting}>
					Next
				</button>
			{:else}
				<button type="submit" class="fm-btn fm-btn--primary" disabled={submitting}>
					{submitting ? 'Sending…' : (settings?.submit_label ?? 'Submit')}
				</button>
			{/if}
		</div>
	{/if}
</form>
