<!--
  FORM-10: Minimal multi-step progress bar.

  Shown in `FormRenderer` when a form has >1 PageBreak-delimited step. The
  filled track width is driven by `aria-valuenow` + CSS var so assistive tech
  sees an authoritative `progressbar` role without us duplicating state.

  Commit 4 extends this with click-to-jump + step labels; commit 1 ships the
  bar so the renderer's multi-step branch compiles cleanly from day one.
-->
<script lang="ts">
	interface Props {
		/** 0-based index of the active step. */
		readonly current: number;
		/** Total number of steps (must be >= 1). */
		readonly total: number;
		/** Optional labels — one per step, rendered as aria-only text. */
		readonly labels?: readonly string[];
	}

	const { current, total, labels }: Props = $props();
	const pct = $derived(total > 1 ? (current / (total - 1)) * 100 : 100);
	const pctStyle = $derived(`--fm-progress-pct: ${pct.toFixed(2)}%`);
</script>

<div
	class="fm-progress"
	role="progressbar"
	aria-valuemin={1}
	aria-valuemax={total}
	aria-valuenow={Math.min(current + 1, total)}
	aria-valuetext={labels?.[current] ?? `Step ${current + 1} of ${total}`}
>
	<p class="fm-progress__label">Step {Math.min(current + 1, total)} of {total}</p>
	<div class="fm-progress__track">
		<div class="fm-progress__fill" style={pctStyle}></div>
	</div>
</div>
