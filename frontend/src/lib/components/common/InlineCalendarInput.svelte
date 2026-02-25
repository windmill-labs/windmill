<script module lang="ts">
	export interface CalendarDate {
		day: number | null
		month: number | null // 1-indexed
		year: number | null
		hour: number | null
		minute: number | null
	}

	export interface CalendarRange {
		start: CalendarDate
		end: CalendarDate
	}

	export function fromCalendarDate(cd: CalendarDate | null | undefined): Date | null {
		if (calendarDateIsNull(cd)) return null
		const now = new Date()
		return new Date(
			cd?.year ?? now.getFullYear(),
			(cd?.month ?? now.getMonth() + 1) - 1,
			cd?.day ?? now.getDate(),
			cd?.hour ?? 0,
			cd?.minute ?? 0
		)
	}
	export function toCalendarDate(date: Date | null | undefined): CalendarDate {
		if (!date) {
			return { day: null, month: null, year: null, hour: null, minute: null }
		}
		return {
			day: date.getDate(),
			month: date.getMonth() + 1,
			year: date.getFullYear(),
			hour: date.getHours(),
			minute: date.getMinutes()
		}
	}
	function calendarDateIsNull(cd: CalendarDate | undefined | null): boolean {
		if (!cd) return true
		return cd.day == null || cd.month == null || cd.year == null
	}
</script>

<script lang="ts">
	import { startOfWeek, endOfWeek, eachDayOfInterval, isSameMonth, getDaysInMonth } from 'date-fns'
	import { ChevronLeft, ChevronRight, ClockIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import Button from './button/Button.svelte'
	import Select from '../select/Select.svelte'
	import { isUSLocale } from '$lib/utils'

	interface DateProps {
		mode?: 'date'
		value?: CalendarDate
	}

	interface RangeProps {
		mode: 'range'
		value?: CalendarRange
		onClickBehavior?: 'set-range' | 'set-start' | 'set-end'
		infiniteRange?: boolean
	}

	type Props = (DateProps | RangeProps) & {
		class?: string
	}

	let { mode = 'date', value = $bindable(), class: className, ...rest }: Props = $props()

	const onClickBehavior = $derived(
		mode === 'range' ? ((rest as RangeProps).onClickBehavior ?? 'set-range') : 'set-range'
	)

	const infiniteRange = $derived(mode === 'range' && !!(rest as RangeProps).infiniteRange)

	const emptyDate: CalendarDate = { day: null, month: null, year: null, hour: null, minute: null }

	export function calendarDateIsNull(cd: CalendarDate | null | undefined): boolean {
		return !cd || (cd.day == null && cd.month == null && cd.year == null)
	}

	// Pick the date that should be visible on mount
	function initialViewDate(): { month: number; year: number } {
		const fallback = { month: today.getMonth() + 1, year: today.getFullYear() }

		let cd: CalendarDate | null | undefined
		if (mode === 'date') {
			cd = value as CalendarDate | undefined
		} else {
			const v = value as CalendarRange | undefined
			const behavior = (rest as RangeProps).onClickBehavior ?? 'set-range'
			if (behavior === 'set-start') {
				cd = v?.start
			} else if (behavior === 'set-end') {
				cd = v?.end
			} else {
				cd = !calendarDateIsNull(v?.start) ? v?.start : v?.end
			}
		}

		if (!calendarDateIsNull(cd) && cd!.month != null && cd!.year != null) {
			return { month: cd!.month, year: cd!.year }
		}
		return fallback
	}

	const today = new Date()

	// Internal navigation state (what month/year is displayed in the calendar)
	const _init = initialViewDate()
	let viewMonth = $state(_init.month)
	let viewYear = $state(_init.year)

	// The date whose month/year the calendar should track (mirrors initialViewDate logic)
	const trackedDate = $derived.by((): CalendarDate | null => {
		if (mode === 'date') {
			const v = value as CalendarDate | undefined
			return v && !calendarDateIsNull(v) ? v : null
		}
		const v = value as CalendarRange | undefined
		if (onClickBehavior === 'set-start')
			return v?.start && !calendarDateIsNull(v.start) ? v.start : null
		if (onClickBehavior === 'set-end') return v?.end && !calendarDateIsNull(v.end) ? v.end : null
		return null
	})

	// Keep the view in sync when value is changed externally
	$effect(() => {
		if (trackedDate?.month != null && trackedDate?.year != null) {
			viewMonth = trackedDate.month
			viewYear = trackedDate.year
		}
	})

	// Range hover tracking
	let hoverDate: CalendarDate | null = $state(null)
	let rangeSelectingStart: boolean = $state(false)

	// Month names for selector
	const MONTH_NAMES = [
		'January',
		'February',
		'March',
		'April',
		'May',
		'June',
		'July',
		'August',
		'September',
		'October',
		'November',
		'December'
	]

	const YEAR_LIST_DELTA = 4
	const YEAR_LIST = Array.from(
		{ length: YEAR_LIST_DELTA * 2 + 1 },
		(_, i) => new Date().getFullYear() - YEAR_LIST_DELTA + i
	)

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
			hour: null as number | null,
			minute: null as number | null,
			isCurrentMonth: isSameMonth(d, firstDay)
		}))
	})

	// Year options for selector (current year ± 50)
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
			if (onClickBehavior === 'set-start') return isSameCalDate(cell, v.start)
			if (onClickBehavior === 'set-end') return isSameCalDate(cell, v.end)
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

		if (infiniteRange) {
			if (!startDate && !endDate) return false
			if (!startDate) return cellDate < endDate!
			if (!endDate) return cellDate > startDate
		}
		if (!startDate || !endDate) return false
		return cellDate > startDate && cellDate < endDate
	}

	function isDayRangeStart(cell: CalendarDate): boolean {
		if (mode !== 'range') return false
		const v = value as CalendarRange | undefined
		return isSameCalDate(cell, v?.start ?? null)
	}

	function isDayDisabled(cell: CalendarDate): boolean {
		if (mode !== 'range') return false
		const v = value as CalendarRange | undefined
		const cellDate = calendarDateToDate(cell)
		if (!cellDate) return false

		if (onClickBehavior === 'set-start') {
			const endDate = v?.end ? calendarDateToDate(v.end) : null
			return endDate != null && cellDate > endDate
		}
		if (onClickBehavior === 'set-end') {
			const startDate = v?.start ? calendarDateToDate(v.start) : null
			return startDate != null && cellDate < startDate
		}
		return false
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
		const cd: CalendarDate = {
			day: cell.day,
			month: cell.month,
			year: cell.year,
			hour: null,
			minute: null
		}

		if (mode === 'date') {
			const v = value as CalendarDate | undefined
			value = isSameCalDate(cd, v ?? null)
				? emptyDate
				: { ...cd, hour: v?.hour ?? null, minute: v?.minute ?? null }
		} else {
			const v = value as CalendarRange | undefined

			if (onClickBehavior === 'set-start') {
				const newStart = isSameCalDate(cd, v?.start ?? null)
					? emptyDate
					: { ...cd, hour: v?.start?.hour ?? null, minute: v?.start?.minute ?? null }
				value = { start: newStart, end: v?.end ?? emptyDate }
			} else if (onClickBehavior === 'set-end') {
				const newEnd = isSameCalDate(cd, v?.end ?? null)
					? emptyDate
					: { ...cd, hour: v?.end?.hour ?? null, minute: v?.end?.minute ?? null }
				value = { start: v?.start ?? emptyDate, end: newEnd }
			} else {
				// 'set-range': two-click flow
				if (!rangeSelectingStart) {
					// First click: toggle start (deselect if same day), clear end
					if (isSameCalDate(cd, v?.start ?? null)) {
						value = { start: emptyDate, end: emptyDate }
					} else {
						value = { start: cd, end: emptyDate }
						rangeSelectingStart = true
					}
				} else {
					// Second click: set end, auto-swap if needed
					const start = v?.start ?? emptyDate
					const startDate = calendarDateToDate(start)
					const endDate = calendarDateToDate(cd)
					if (startDate && endDate && endDate < startDate) {
						value = { start: cd, end: start }
					} else {
						value = { start, end: cd }
					}
					rangeSelectingStart = false
					hoverDate = null
				}
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

	// The CalendarDate whose hour/minute the time inputs control
	const timeTarget = $derived.by((): CalendarDate | null => {
		if (mode === 'date') return (value as CalendarDate | undefined) ?? null
		const v = value as CalendarRange | undefined
		if (onClickBehavior === 'set-start') return v?.start ?? null
		if (onClickBehavior === 'set-end') return v?.end ?? null
		return null // set-range: no single time target
	})

	const showTime = $derived(timeTarget !== null)

	// Returns the base CalendarDate to mutate, falling back to today if no date selected yet
	function withTodayFallback(cd: CalendarDate): CalendarDate {
		if (cd.day != null && cd.month != null && cd.year != null) return cd
		const t = new Date()
		return {
			...cd,
			day: cd.day ?? t.getDate(),
			month: cd.month ?? t.getMonth() + 1,
			year: cd.year ?? t.getFullYear()
		}
	}

	const usLocale = $derived(isUSLocale())

	function selectAllOnFocus(e: FocusEvent) {
		;(e.target as HTMLInputElement).select()
	}

	function patchTarget(patch: Partial<CalendarDate>) {
		if (mode === 'date') {
			value = { ...withTodayFallback(value as CalendarDate), ...patch }
		} else {
			const v = value as CalendarRange
			if (onClickBehavior === 'set-start')
				value = { start: { ...withTodayFallback(v.start), ...patch }, end: v.end }
			else if (onClickBehavior === 'set-end')
				value = { start: v.start, end: { ...withTodayFallback(v.end), ...patch } }
		}
	}

	function setDay(raw: string) {
		const d = parseInt(raw, 10)
		if (isNaN(d)) return
		patchTarget({ day: Math.max(1, Math.min(31, d)) })
	}

	function setMonth(raw: string) {
		const mo = parseInt(raw, 10)
		if (isNaN(mo)) return
		patchTarget({ month: Math.max(1, Math.min(12, mo)) })
	}

	function setYear(raw: string) {
		const y = parseInt(raw, 10)
		if (isNaN(y)) return
		patchTarget({ year: y })
	}
</script>

<div class="flex flex-col {className}">
	<!-- Header -->
	<div class="mb-3 flex items-center gap-1.5">
		<Button
			endIcon={{ icon: ChevronLeft }}
			iconOnly
			unifiedSize="md"
			onClick={prevMonth}
			wrapperClasses="bg-surface-input"
		/>

		<div class="flex flex-1 divide-x">
			<Select
				class="basis-1/2"
				inputClass="text-center !rounded-r-none !border-r-0"
				disablePortal
				bind:value={viewMonth}
				items={MONTH_NAMES.map((name, i) => ({ label: name, value: i + 1 }))}
			/>
			<Select
				class="basis-1/2"
				inputClass="text-center !rounded-l-none !border-l-0"
				disablePortal
				bind:value={viewYear}
				onCreateItem={(val) => (viewYear = parseInt(val) || viewYear)}
				items={YEAR_LIST.map((year) => ({ label: year.toString(), value: year }))}
			/>
		</div>

		<Button
			endIcon={{ icon: ChevronRight }}
			iconOnly
			unifiedSize="md"
			onClick={nextMonth}
			wrapperClasses="bg-surface-input"
		/>
	</div>

	<!-- Day-of-week labels -->
	<div class="mb-1 grid grid-cols-7">
		{#each DAY_LABELS as label (label)}
			<div
				class="flex h-7 items-center justify-center text-3xs font-medium uppercase tracking-wide text-secondary"
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
			{@const disabled = isDayDisabled(cell)}
			{@const isToday =
				cell.day === today.getDate() &&
				cell.month === today.getMonth() + 1 &&
				cell.year === today.getFullYear()}
			<button
				type="button"
				onclick={() => !disabled && handleDayClick(cell)}
				onmouseenter={() => {
					if (rangeSelectingStart) hoverDate = cell
				}}
				onmouseleave={() => {
					if (rangeSelectingStart) hoverDate = null
				}}
				class={twMerge(
					'relative flex my-0.5 h-9 min-w-9 w-full items-center justify-center text-sm transition-colors focus:outline-none',
					disabled ? 'opacity-15' : !cell.isCurrentMonth ? 'text-hint' : 'text-primary',
					!disabled && selected ? 'font-semibold' : 'font-normal',
					!disabled && inRange ? 'bg-surface-secondary' : '',
					!disabled && (isStart || isEnd) ? 'bg-surface-secondary' : '',
					!disabled && isStart && mode === 'range' ? 'rounded-l' : '',
					!disabled && isEnd && mode === 'range' ? 'rounded-r' : ''
				)}
				aria-label="{cell.year}-{String(cell.month).padStart(2, '0')}-{String(cell.day).padStart(
					2,
					'0'
				)}"
				aria-pressed={selected}
				aria-disabled={disabled}
			>
				<!-- Selection circle / highlight -->
				{#if selected}
					<span class="absolute inset-0 z-0 rounded-md bg-surface-accent-primary"></span>
				{/if}
				<span
					class={twMerge(
						'relative z-10',
						selected
							? 'text-white dark:text-white'
							: isToday && !disabled
								? 'text-accent font-semibold'
								: ''
					)}
				>
					{cell.day}
				</span>
			</button>
		{/each}
	</div>

	<div class="flex-1"></div>

	<!-- Time inputs -->
	{#if showTime}
		<div class="border-t my-4"></div>
		<div class="flex justify-center">
			<div
				class="px-2 !h-8 flex border bg-surface-secondary dark:bg-surface rounded-l-md w-fit items-center gap-0"
			>
				{#if usLocale}
					<!-- MM / DD / YYYY -->
					<input
						type="text"
						inputmode="numeric"
						maxlength="2"
						value={timeTarget?.month != null ? String(timeTarget.month).padStart(2, '0') : ''}
						placeholder="MM"
						onfocus={selectAllOnFocus}
						onchange={(e) => setMonth((e.target as HTMLInputElement).value)}
						style="background: transparent !important;"
						class="!border-none !w-8 !h-7 !px-1.5 text-center font-mono"
						aria-label="Month"
					/>
					<span class="text-sm font-medium font-mono text-secondary">/</span>
					<input
						type="text"
						inputmode="numeric"
						maxlength="2"
						value={timeTarget?.day != null ? String(timeTarget.day).padStart(2, '0') : ''}
						placeholder="DD"
						onfocus={selectAllOnFocus}
						onchange={(e) => setDay((e.target as HTMLInputElement).value)}
						style="background: transparent !important;"
						class="!border-none !w-8 !h-7 !px-1.5 text-center font-mono"
						aria-label="Day"
					/>
				{:else}
					<!-- DD / MM / YYYY -->
					<input
						type="text"
						inputmode="numeric"
						maxlength="2"
						value={timeTarget?.day != null ? String(timeTarget.day).padStart(2, '0') : ''}
						placeholder="DD"
						onfocus={selectAllOnFocus}
						onchange={(e) => setDay((e.target as HTMLInputElement).value)}
						style="background: transparent !important;"
						class="!border-none !w-8 !h-7 !px-1.5 text-center font-mono"
						aria-label="Day"
					/>
					<span class="text-sm font-medium font-mono text-secondary">/</span>
					<input
						type="text"
						inputmode="numeric"
						maxlength="2"
						value={timeTarget?.month != null ? String(timeTarget.month).padStart(2, '0') : ''}
						placeholder="MM"
						onfocus={selectAllOnFocus}
						onchange={(e) => setMonth((e.target as HTMLInputElement).value)}
						style="background: transparent !important;"
						class="!border-none !w-8 !h-7 !px-1.5 text-center font-mono"
						aria-label="Month"
					/>
				{/if}
				<span class="text-sm font-medium font-mono text-secondary">/</span>
				<input
					type="text"
					inputmode="numeric"
					maxlength="4"
					value={timeTarget?.year != null ? String(timeTarget.year) : ''}
					placeholder="YYYY"
					onfocus={selectAllOnFocus}
					onchange={(e) => setYear((e.target as HTMLInputElement).value)}
					style="background: transparent !important;"
					class="!border-none !w-12 !h-7 !px-1.5 text-center font-mono"
					aria-label="Year"
				/>
			</div>
			<div
				class="pl-2 !h-8 flex border border-l-0 rounded-r-md w-fit items-center gap-0 bg-surface-input"
			>
				<input
					type="text"
					inputmode="numeric"
					maxlength="2"
					value={timeTarget?.hour != null ? String(timeTarget.hour).padStart(2, '0') : ''}
					placeholder="HH"
					onfocus={selectAllOnFocus}
					onchange={(e) => {
						const h = Math.max(0, Math.min(23, parseInt((e.target as HTMLInputElement).value, 10)))
						if (!isNaN(h)) patchTarget({ hour: h })
					}}
					class="!border-none !w-8 !h-7 !px-1.5 text-right font-mono"
					aria-label="Hour"
				/>
				<span class="text-sm font-medium font-mono text-secondary">:</span>
				<input
					type="text"
					inputmode="numeric"
					maxlength="2"
					value={timeTarget?.minute != null ? String(timeTarget.minute).padStart(2, '0') : ''}
					placeholder="MM"
					onfocus={selectAllOnFocus}
					onchange={(e) => {
						const m = Math.max(0, Math.min(59, parseInt((e.target as HTMLInputElement).value, 10)))
						if (!isNaN(m)) patchTarget({ minute: m })
					}}
					class="!border-none !w-8 !h-7 !px-1.5 text-left font-mono"
					aria-label="Minute"
				/>
				<ClockIcon size={14} class="mr-3" />
			</div>
		</div>
	{/if}
</div>
