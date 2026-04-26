<script lang="ts">
	import {
		format,
		startOfMonth,
		endOfMonth,
		startOfWeek,
		endOfWeek,
		addDays,
		addMonths,
		subMonths,
		isSameDay,
		isSameMonth,
		isAfter,
		isBefore,
		subDays,
		startOfYear
	} from 'date-fns';
	import CalendarBlankIcon from 'phosphor-svelte/lib/CalendarBlankIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';

	interface DatePreset {
		label: string;
		getRange: () => { start: Date; end: Date };
	}

	interface Props {
		startDate: Date | null;
		endDate: Date | null;
		presets?: DatePreset[];
	}

	let {
		startDate = $bindable(null),
		endDate = $bindable(null),
		presets
	}: Props = $props();

	const defaultPresets: DatePreset[] = [
		{
			label: 'Last 7 days',
			getRange: () => ({ start: subDays(new Date(), 6), end: new Date() })
		},
		{
			label: 'Last 30 days',
			getRange: () => ({ start: subDays(new Date(), 29), end: new Date() })
		},
		{
			label: 'Last 90 days',
			getRange: () => ({ start: subDays(new Date(), 89), end: new Date() })
		},
		{
			label: 'This month',
			getRange: () => ({ start: startOfMonth(new Date()), end: new Date() })
		},
		{
			label: 'Last month',
			getRange: () => {
				const lastMonth = subMonths(new Date(), 1);
				return { start: startOfMonth(lastMonth), end: endOfMonth(lastMonth) };
			}
		},
		{
			label: 'This year',
			getRange: () => ({ start: startOfYear(new Date()), end: new Date() })
		}
	];

	const activePresets = $derived(presets ?? defaultPresets);

	let isOpen = $state(false);
	let leftMonth = $state(subMonths(new Date(), 1));
	let rightMonth = $state(new Date());

	// Internal selection state (applied on "Apply")
	let selStart = $state<Date | null>(startDate);
	let selEnd = $state<Date | null>(endDate);
	let hoveredDate = $state<Date | null>(null);

	// Whether we're selecting the start or end date
	let selectingEnd = $state(false);

	const WEEKDAYS = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];

	function getCalendarDays(month: Date): Date[] {
		const monthStart = startOfMonth(month);
		const monthEnd = endOfMonth(month);
		const calStart = startOfWeek(monthStart);
		const calEnd = endOfWeek(monthEnd);

		const days: Date[] = [];
		let day = calStart;
		while (!isAfter(day, calEnd)) {
			days.push(day);
			day = addDays(day, 1);
		}
		return days;
	}

	const leftDays = $derived(getCalendarDays(leftMonth));
	const rightDays = $derived(getCalendarDays(rightMonth));

	const displayText = $derived.by(() => {
		if (startDate && endDate) {
			return `${format(startDate, 'MMM d, yyyy')} - ${format(endDate, 'MMM d, yyyy')}`;
		}
		return 'Select date range';
	});

	function handleDayClick(day: Date) {
		if (!selectingEnd) {
			selStart = day;
			selEnd = null;
			selectingEnd = true;
		} else {
			if (selStart && isBefore(day, selStart)) {
				selEnd = selStart;
				selStart = day;
			} else {
				selEnd = day;
			}
			selectingEnd = false;
		}
	}

	function isInRange(day: Date): boolean {
		if (!selStart) return false;
		const rangeEnd = selEnd ?? hoveredDate;
		if (!rangeEnd) return false;

		const start = isBefore(selStart, rangeEnd) ? selStart : rangeEnd;
		const end = isAfter(selStart, rangeEnd) ? selStart : rangeEnd;

		return isAfter(day, start) && isBefore(day, end);
	}

	function isRangeStart(day: Date): boolean {
		if (!selStart) return false;
		return isSameDay(day, selStart);
	}

	function isRangeEnd(day: Date): boolean {
		const rangeEnd = selEnd ?? hoveredDate;
		if (!rangeEnd) return false;
		// If hoveredDate is before selStart, the "end" visually is selStart
		if (selStart && !selEnd && hoveredDate && isBefore(hoveredDate, selStart)) {
			return isSameDay(day, selStart);
		}
		return isSameDay(day, rangeEnd);
	}

	function applyPreset(preset: DatePreset) {
		const range = preset.getRange();
		selStart = range.start;
		selEnd = range.end;
		selectingEnd = false;
	}

	function handleApply() {
		startDate = selStart;
		endDate = selEnd;
		isOpen = false;
	}

	function handleCancel() {
		selStart = startDate;
		selEnd = endDate;
		selectingEnd = false;
		isOpen = false;
	}

	function toggleOpen() {
		if (!isOpen) {
			selStart = startDate;
			selEnd = endDate;
			selectingEnd = false;
			if (startDate) {
				leftMonth = startOfMonth(subMonths(startDate, 0));
				rightMonth = addMonths(leftMonth, 1);
			} else {
				leftMonth = subMonths(new Date(), 1);
				rightMonth = new Date();
			}
		}
		isOpen = !isOpen;
	}

	function prevMonth() {
		leftMonth = subMonths(leftMonth, 1);
		rightMonth = subMonths(rightMonth, 1);
	}

	function nextMonth() {
		leftMonth = addMonths(leftMonth, 1);
		rightMonth = addMonths(rightMonth, 1);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && isOpen) {
			handleCancel();
		}
	}

	function handleBackdropClick() {
		handleCancel();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="drp">
	<button class="drp__trigger" onclick={toggleOpen} type="button">
		<span class="drp__trigger-icon">
			<CalendarBlankIcon size={16} weight="bold" />
		</span>
		<span class="drp__trigger-text">{displayText}</span>
		<span class="drp__trigger-chevron" class:drp__trigger-chevron--open={isOpen}>
			<CaretDownIcon size={14} weight="bold" />
		</span>
	</button>

	{#if isOpen}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="drp__backdrop" onclick={handleBackdropClick} onkeydown={() => {}}></div>
		<div class="drp__popover">
			<div class="drp__presets">
				{#each activePresets as preset (preset.label)}
					<button class="drp__preset-btn" onclick={() => applyPreset(preset)} type="button">
						{preset.label}
					</button>
				{/each}
				<button
					class="drp__preset-btn drp__preset-btn--active"
					type="button"
					disabled
				>
					Custom
				</button>
			</div>

			<div class="drp__calendars">
				<div class="drp__calendar">
					<div class="drp__calendar-header">
						<button class="drp__nav-btn" onclick={prevMonth} type="button" aria-label="Previous month">
							<CaretLeftIcon size={14} weight="bold" />
						</button>
						<span class="drp__month-label">{format(leftMonth, 'MMMM yyyy')}</span>
						<span class="drp__nav-spacer"></span>
					</div>
					<div class="drp__weekdays">
						{#each WEEKDAYS as day (day)}
							<span class="drp__weekday">{day}</span>
						{/each}
					</div>
					<div class="drp__days">
						{#each leftDays as day (day.getTime())}
							<button
								class="drp__day"
								class:drp__day--outside={!isSameMonth(day, leftMonth)}
								class:drp__day--selected={isRangeStart(day) || isRangeEnd(day)}
								class:drp__day--in-range={isInRange(day)}
								class:drp__day--range-start={isRangeStart(day)}
								class:drp__day--range-end={isRangeEnd(day)}
								class:drp__day--today={isSameDay(day, new Date())}
								onclick={() => handleDayClick(day)}
								onmouseenter={() => { hoveredDate = day; }}
								onmouseleave={() => { hoveredDate = null; }}
								type="button"
							>
								{format(day, 'd')}
							</button>
						{/each}
					</div>
				</div>

				<div class="drp__calendar">
					<div class="drp__calendar-header">
						<span class="drp__nav-spacer"></span>
						<span class="drp__month-label">{format(rightMonth, 'MMMM yyyy')}</span>
						<button class="drp__nav-btn" onclick={nextMonth} type="button" aria-label="Next month">
							<CaretRightIcon size={14} weight="bold" />
						</button>
					</div>
					<div class="drp__weekdays">
						{#each WEEKDAYS as day (day)}
							<span class="drp__weekday">{day}</span>
						{/each}
					</div>
					<div class="drp__days">
						{#each rightDays as day (day.getTime())}
							<button
								class="drp__day"
								class:drp__day--outside={!isSameMonth(day, rightMonth)}
								class:drp__day--selected={isRangeStart(day) || isRangeEnd(day)}
								class:drp__day--in-range={isInRange(day)}
								class:drp__day--range-start={isRangeStart(day)}
								class:drp__day--range-end={isRangeEnd(day)}
								class:drp__day--today={isSameDay(day, new Date())}
								onclick={() => handleDayClick(day)}
								onmouseenter={() => { hoveredDate = day; }}
								onmouseleave={() => { hoveredDate = null; }}
								type="button"
							>
								{format(day, 'd')}
							</button>
						{/each}
					</div>
				</div>
			</div>

			<div class="drp__footer">
				{#if selStart}
					<span class="drp__selection-label">
						{format(selStart, 'MMM d, yyyy')}
						{#if selEnd}
							&mdash; {format(selEnd, 'MMM d, yyyy')}
						{/if}
					</span>
				{:else}
					<span class="drp__selection-label drp__selection-label--empty">Pick a start date</span>
				{/if}
				<div class="drp__footer-actions">
					<button class="drp__footer-btn drp__footer-btn--cancel" onclick={handleCancel} type="button">
						Cancel
					</button>
					<button
						class="drp__footer-btn drp__footer-btn--apply"
						onclick={handleApply}
						disabled={!selStart || !selEnd}
						type="button"
					>
						Apply
					</button>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.drp {
		position: relative;
		display: inline-block;
		font-family: var(--font-ui);
	}

	.drp__trigger {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2-5) var(--space-4);
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		color: var(--color-off-white);
		font-family: var(--font-ui);
		font-size: var(--fs-sm);
		cursor: pointer;
		transition: all var(--duration-200) var(--ease-out);
		white-space: nowrap;
	}

	.drp__trigger:hover {
		border-color: rgba(255, 255, 255, 0.2);
		background: var(--color-deep-blue);
	}

	.drp__trigger:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-navy),
			0 0 0 4px rgba(15, 164, 175, 0.7);
	}

	.drp__trigger-icon {
		display: inline-flex;
		flex-shrink: 0;
		color: var(--color-teal);
	}

	.drp__trigger-text {
		flex: 1;
	}

	.drp__trigger-chevron {
		display: inline-flex;
		flex-shrink: 0;
		color: var(--color-grey-400);
		transition: transform var(--duration-200) var(--ease-out);
	}

	.drp__trigger-chevron--open {
		transform: rotate(180deg);
	}

	.drp__backdrop {
		position: fixed;
		inset: 0;
		z-index: var(--z-40);
	}

	.drp__popover {
		position: absolute;
		top: calc(100% + var(--space-2));
		left: 0;
		z-index: var(--z-50);
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		box-shadow: var(--shadow-2xl);
		overflow: hidden;
		animation: drp-open var(--duration-200) var(--ease-out) forwards;
	}

	@keyframes drp-open {
		from {
			opacity: 0;
			transform: translateY(-8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.drp__presets {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.drp__preset-btn {
		padding: var(--space-1) var(--space-3);
		border: none;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
		font-family: var(--font-ui);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
		white-space: nowrap;
	}

	.drp__preset-btn:hover:not(:disabled) {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	.drp__preset-btn--active {
		background: rgba(15, 164, 175, 0.2);
		color: var(--color-teal);
	}

	.drp__calendars {
		display: flex;
		gap: var(--space-2);
		padding: var(--space-4);
	}

	.drp__calendar {
		min-width: 252px;
	}

	.drp__calendar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-3);
		padding: 0 var(--space-1);
	}

	.drp__month-label {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		text-align: center;
		flex: 1;
	}

	.drp__nav-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-400);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
	}

	.drp__nav-btn:hover {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}

	.drp__nav-spacer {
		width: 28px;
	}

	.drp__weekdays {
		display: grid;
		grid-template-columns: repeat(7, 1fr);
		margin-bottom: var(--space-1);
	}

	.drp__weekday {
		text-align: center;
		font-size: var(--fs-2xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-500);
		padding: var(--space-1) 0;
	}

	.drp__days {
		display: grid;
		grid-template-columns: repeat(7, 1fr);
	}

	.drp__day {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-200);
		font-family: var(--font-ui);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
		position: relative;
	}

	.drp__day:hover:not(.drp__day--selected) {
		background: rgba(255, 255, 255, 0.08);
	}

	.drp__day--outside {
		color: var(--color-grey-600);
	}

	.drp__day--today {
		font-weight: var(--w-bold);
		color: var(--color-teal);
	}

	.drp__day--in-range {
		background: rgba(15, 164, 175, 0.1);
		border-radius: 0;
		color: var(--color-teal-light);
	}

	.drp__day--range-start {
		background: var(--color-teal);
		color: var(--color-white);
		border-radius: var(--radius-md) 0 0 var(--radius-md);
	}

	.drp__day--range-end {
		background: var(--color-teal);
		color: var(--color-white);
		border-radius: 0 var(--radius-md) var(--radius-md) 0;
	}

	.drp__day--selected.drp__day--range-start.drp__day--range-end {
		border-radius: var(--radius-md);
	}

	.drp__day--selected {
		background: var(--color-teal);
		color: var(--color-white);
		font-weight: var(--w-semibold);
	}

	.drp__footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-3) var(--space-4);
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.drp__selection-label {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
		font-weight: var(--w-medium);
	}

	.drp__selection-label--empty {
		color: var(--color-grey-500);
	}

	.drp__footer-actions {
		display: flex;
		gap: var(--space-2);
	}

	.drp__footer-btn {
		padding: var(--space-1-5) var(--space-4);
		border: none;
		border-radius: var(--radius-lg);
		font-family: var(--font-ui);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
	}

	.drp__footer-btn--cancel {
		background: transparent;
		color: var(--color-grey-400);
	}

	.drp__footer-btn--cancel:hover {
		color: var(--color-white);
	}

	.drp__footer-btn--apply {
		background: var(--color-teal);
		color: var(--color-white);
		box-shadow: 0 2px 8px rgba(15, 164, 175, 0.25);
	}

	.drp__footer-btn--apply:hover:not(:disabled) {
		background: var(--color-teal-light);
	}

	.drp__footer-btn--apply:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	/* Mobile: stack calendars */
	@media (max-width: 600px) {
		.drp__popover {
			position: fixed;
			top: auto;
			bottom: 0;
			left: 0;
			right: 0;
			border-radius: var(--radius-2xl) var(--radius-2xl) 0 0;
			max-height: 90vh;
			overflow-y: auto;
		}

		.drp__calendars {
			flex-direction: column;
			gap: var(--space-4);
		}

		.drp__calendar {
			min-width: auto;
		}

		.drp__day {
			width: 100%;
		}

		.drp__days {
			gap: var(--space-0-5);
		}

		.drp__presets {
			overflow-x: auto;
			flex-wrap: nowrap;
			-webkit-overflow-scrolling: touch;
		}
	}
</style>
