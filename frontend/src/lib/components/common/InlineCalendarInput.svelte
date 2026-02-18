<script lang="ts">
	import { startOfWeek, endOfWeek, eachDayOfInterval, isSameMonth, getDaysInMonth } from 'date-fns'
	import { twMerge } from 'tailwind-merge'

	export interface CalendarDate {
		day: number | null
		month: number | null // 1-indexed
		year: number | null
	}

	export interface CalendarRange {
		start: CalendarDate
		end: CalendarDate
	}

	interface DateProps {
		mode?: 'date'
		value?: CalendarDate
	}

	interface RangeProps {
		mode: 'range'
		value?: CalendarRange
	}

	type Props = DateProps | RangeProps

	let { mode = 'date', value = $bindable() }: Props = $props()

	const emptyDate: CalendarDate = { day: null, month: null, year: null }

	// Internal navigation state (what month/year is displayed in the calendar)
	const today = new Date()
	let viewMonth = $state(today.getMonth() + 1) // 1-indexed
	let viewYear = $state(today.getFullYear())

	// Range hover tracking
	let hoverDate: CalendarDate | null = $state(null)
	let rangeSelectingStart: boolean = $state(false)

	// Month names for selector
	const MONTH_NAMES = [
		'Jan',
		'Feb',
		'Mar',
		'Apr',
		'May',
		'Jun',
		'Jul',
		'Aug',
		'Sep',
		'Oct',
		'Nov',
		'Dec'
	]

	const DAY_LABELS = ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT']

	// Build the grid: 6 rows × 7 cols of CalendarDate (with nullish fields for padding)
	const calendarDays = $derived.by(() => {
		const firstDay = new Date(viewYear, viewMonth - 1, 1)
		const lastDay = new Date(viewYear, viewMonth - 1, getDaysInMonth(firstDay))
		const gridStart = startOfWeek(firstDay, { weekStartsOn: 0 })
		const gridEnd = endOfWeek(lastDay, { weekStartsOn: 0 })
		const allDays = eachDayOfInterval({ start: gridStart, end: gridEnd })
		return allDays.map((d) => ({
			day: d.getDate(),
			month: d.getMonth() + 1,
			year: d.getFullYear(),
			isCurrentMonth: isSameMonth(d, firstDay)
		}))
	})

	// Year options for selector (current year ± 50)
	const yearOptions = $derived.by(() => {
		const years: number[] = []
		for (let y = viewYear - 50; y <= viewYear + 50; y++) years.push(y)
		return years
	})

	function calendarDateToDate(cd: CalendarDate): Date | null {
		if (cd.day == null || cd.month == null || cd.year == null) return null
		return new Date(cd.year, cd.month - 1, cd.day)
	}

	function isSameCalDate(a: CalendarDate | null, b: CalendarDate | null): boolean {
		if (!a || !b) return false
		return a.day === b.day && a.month === b.month && a.year === b.year
	}

	function isDaySelected(cell: CalendarDate): boolean {
		if (mode === 'date') {
			const v = value as CalendarDate | undefined
			if (!v) return false
			return isSameCalDate(cell, v)
		} else {
			const v = value as CalendarRange | undefined
			if (!v) return false
			return isSameCalDate(cell, v.start) || isSameCalDate(cell, v.end)
		}
	}

	function isDayInRange(cell: CalendarDate): boolean {
		if (mode !== 'range') return false
		const v = value as CalendarRange | undefined
		const startDate = v?.start ? calendarDateToDate(v.start) : null
		const endDate = v?.end ? calendarDateToDate(v.end) : null
		const cellDate = calendarDateToDate(cell)
		if (!cellDate) return false

		// If actively selecting (first click done, hovering), show hover range
		if (rangeSelectingStart && startDate && hoverDate) {
			const hDate = calendarDateToDate(hoverDate)
			if (hDate) {
				const lo = startDate < hDate ? startDate : hDate
				const hi = startDate < hDate ? hDate : startDate
				return cellDate > lo && cellDate < hi
			}
		}

		if (!startDate || !endDate) return false
		return cellDate > startDate && cellDate < endDate
	}

	function isDayRangeStart(cell: CalendarDate): boolean {
		if (mode !== 'range') return false
		const v = value as CalendarRange | undefined
		return isSameCalDate(cell, v?.start ?? null)
	}

	function isDayRangeEnd(cell: CalendarDate): boolean {
		if (mode !== 'range') return false
		const v = value as CalendarRange | undefined
		if (rangeSelectingStart && hoverDate) {
			const startDate = v?.start ? calendarDateToDate(v.start) : null
			const hDate = calendarDateToDate(hoverDate)
			const cellDate = calendarDateToDate(cell)
			if (startDate && hDate && cellDate) {
				const end = startDate < hDate ? hoverDate : (v?.start ?? null)
				return isSameCalDate(cell, end)
			}
		}
		return isSameCalDate(cell, v?.end ?? null)
	}

	function handleDayClick(cell: {
		day: number
		month: number
		year: number
		isCurrentMonth: boolean
	}) {
		const cd: CalendarDate = { day: cell.day, month: cell.month, year: cell.year }

		if (mode === 'date') {
			value = cd
		} else {
			const v = value as CalendarRange | undefined
			if (!rangeSelectingStart) {
				// First click: set start, clear end, begin range selection
				const newRange: CalendarRange = { start: cd, end: emptyDate }
				value = newRange
				rangeSelectingStart = true
			} else {
				// Second click: set end
				const start = v?.start ?? emptyDate
				const startDate = calendarDateToDate(start)
				const endDate = calendarDateToDate(cd)
				let newRange: CalendarRange
				if (startDate && endDate && endDate < startDate) {
					newRange = { start: cd, end: start }
				} else {
					newRange = { start, end: cd }
				}
				value = newRange
				rangeSelectingStart = false
				hoverDate = null
			}
		}

		// Navigate to that month if clicking an out-of-month day
		if (!cell.isCurrentMonth) {
			viewMonth = cell.month
			viewYear = cell.year
		}
	}

	function prevMonth() {
		if (viewMonth === 1) {
			viewMonth = 12
			viewYear -= 1
		} else {
			viewMonth -= 1
		}
	}

	function nextMonth() {
		if (viewMonth === 12) {
			viewMonth = 1
			viewYear += 1
		} else {
			viewMonth += 1
		}
	}

	function onMonthChange(e: Event) {
		viewMonth = parseInt((e.target as HTMLSelectElement).value, 10)
	}

	function onYearChange(e: Event) {
		viewYear = parseInt((e.target as HTMLSelectElement).value, 10)
	}
</script>

<div
	class="inline-block select-none rounded-md border border-surface-secondary bg-surface p-3 shadow-sm"
>
	<!-- Header -->
	<div class="mb-3 flex items-center gap-1">
		<button
			type="button"
			onclick={prevMonth}
			class="flex h-7 w-7 items-center justify-center rounded-md hover:bg-surface-hover text-primary transition-colors"
			aria-label="Previous month"
		>
			<svg
				width="16"
				height="16"
				viewBox="0 0 16 16"
				fill="none"
				xmlns="http://www.w3.org/2000/svg"
			>
				<path
					d="M10 12L6 8L10 4"
					stroke="currentColor"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"
				/>
			</svg>
		</button>

		<div class="flex flex-1 items-center justify-center gap-1">
			<select
				value={viewMonth}
				onchange={onMonthChange}
				class="cursor-pointer rounded-md border-none bg-transparent px-1 py-0.5 text-sm font-medium text-primary hover:bg-surface-hover focus:outline-none focus:ring-1 focus:ring-frost-400"
				aria-label="Select month"
			>
				{#each MONTH_NAMES as name, i (i)}
					<option value={i + 1}>{name}</option>
				{/each}
			</select>

			<select
				value={viewYear}
				onchange={onYearChange}
				class="cursor-pointer rounded-md border-none bg-transparent px-1 py-0.5 text-sm font-medium text-primary hover:bg-surface-hover focus:outline-none focus:ring-1 focus:ring-frost-400"
				aria-label="Select year"
			>
				{#each yearOptions as y (y)}
					<option value={y}>{y}</option>
				{/each}
			</select>
		</div>

		<button
			type="button"
			onclick={nextMonth}
			class="flex h-7 w-7 items-center justify-center rounded-md hover:bg-surface-hover text-primary transition-colors"
			aria-label="Next month"
		>
			<svg
				width="16"
				height="16"
				viewBox="0 0 16 16"
				fill="none"
				xmlns="http://www.w3.org/2000/svg"
			>
				<path
					d="M6 4L10 8L6 12"
					stroke="currentColor"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"
				/>
			</svg>
		</button>
	</div>

	<!-- Day-of-week labels -->
	<div class="mb-1 grid grid-cols-7">
		{#each DAY_LABELS as label (label)}
			<div
				class="flex h-7 items-center justify-center text-[10px] font-semibold uppercase tracking-wide text-secondary"
			>
				{label}
			</div>
		{/each}
	</div>

	<!-- Day grid -->
	<div class="grid grid-cols-7">
		{#each calendarDays as cell (`${cell.year}-${cell.month}-${cell.day}`)}
			{@const selected = isDaySelected(cell)}
			{@const inRange = isDayInRange(cell)}
			{@const isStart = isDayRangeStart(cell)}
			{@const isEnd = isDayRangeEnd(cell)}
			<button
				type="button"
				onclick={() => handleDayClick(cell)}
				onmouseenter={() => {
					if (rangeSelectingStart) hoverDate = cell
				}}
				onmouseleave={() => {
					if (rangeSelectingStart) hoverDate = null
				}}
				class={twMerge(
					'relative flex my-0.5 h-9 w-9 items-center justify-center text-sm transition-colors focus:outline-none',
					!cell.isCurrentMonth ? 'text-tertiary' : 'text-primary',
					selected ? 'font-semibold bg-surface-accent-secondary' : 'font-normal',
					inRange ? 'bg-surface-secondary' : '',
					isStart || isEnd ? 'bg-surface-secondary' : '',
					isStart && mode === 'range' ? 'rounded-l' : '',
					isEnd && mode === 'range' ? 'rounded-r' : ''
				)}
				aria-label="{cell.year}-{String(cell.month).padStart(2, '0')}-{String(cell.day).padStart(
					2,
					'0'
				)}"
				aria-pressed={selected}
			>
				<!-- Selection circle / highlight -->
				{#if selected}
					<span class="absolute inset-0 z-0 rounded-md bg-surface-accent-primary"></span>
				{/if}
				<span class="relative z-10 {selected ? 'text-white dark:text-white' : ''}">
					{cell.day}
				</span>
			</button>
		{/each}
	</div>
</div>
