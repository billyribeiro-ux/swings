<!--
  FORM-10 (multi-step): step progress bar.

  Renders one pill per step + a filled connector track so users can see
  where they are in the flow. The component stays passive — it never
  navigates — because the FormRenderer owns step transitions (which
  depend on per-step validation and save+resume persistence).

  A11y:
  - Top-level `role="progressbar"` with aria-valuemin / valuemax / valuenow
    so SR users hear "Step 2 of 4" announcements at every step change.
  - Individual pills carry aria-current="step" when active.
  - The human-readable "Step N of M" label is duplicated in text so sighted
    users get the same orientation assistive tech does.
-->
<script lang="ts">
	interface Props {
		/** 0-based index of the active step. */
		readonly current: number;
		/** Total number of steps (must be >= 1). */
		readonly total: number;
		/** Optional per-step labels; when omitted the bar shows numeric pills. */
		readonly labels?: readonly string[];
	}

	const { current, total, labels }: Props = $props();
	const pct = $derived(total > 1 ? (current / (total - 1)) * 100 : 100);
	const pctStyle = $derived(`--fm-progress-pct: ${pct.toFixed(2)}%`);
	const stepList = $derived(Array.from({ length: total }, (_, i) => i));
</script>

<div
	class="fm-progress"
	role="progressbar"
	aria-valuemin={1}
	aria-valuemax={total}
	aria-valuenow={Math.min(current + 1, total)}
	aria-valuetext={labels?.[current] ?? `Step ${current + 1} of ${total}`}
>
	<p class="fm-progress__label">
		Step {Math.min(current + 1, total)} of {total}{labels?.[current] ? ` — ${labels[current]}` : ''}
	</p>
	<div class="fm-progress__track">
		<div class="fm-progress__fill" style={pctStyle}></div>
	</div>
	<ol class="fm-progress__steps" aria-hidden="true">
		{#each stepList as i (i)}
			<li
				class="fm-progress__step"
				class:fm-progress__step--done={i < current}
				class:fm-progress__step--active={i === current}
				aria-current={i === current ? 'step' : undefined}
			>
				<span class="fm-progress__step-index">{i + 1}</span>
				{#if labels?.[i]}
					<span class="fm-progress__step-label">{labels[i]}</span>
				{/if}
			</li>
		{/each}
	</ol>
</div>
