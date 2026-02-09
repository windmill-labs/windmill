<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { format, isValid, parse } from 'date-fns'
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()

	interface Props {
		value?: string | null | undefined
		autofocus?: boolean | null
		minDate?: string | undefined
		maxDate?: string | undefined
		dateFormat?: string | undefined
		disabled?: boolean
	}

	let {
		value = $bindable(undefined),
		autofocus = false,
		minDate = undefined,
		maxDate = undefined,
		dateFormat = undefined,
		disabled = false
	}: Props = $props()

	const legacyDateFormat = 'dd-MM-yyyy'
	const isoDateFormat = 'yyyy-MM-dd'

	const dispatch = createEventDispatcher()

	function computeDate(v: string | null | undefined) {
		if (v && v.length > 0) {
			try {
				let parsedDate = parse(v, isoDateFormat, new Date())
				if (parsedDate.toString() === 'Invalid Date') {
					parsedDate = parse(v, legacyDateFormat, new Date())
				}
				return format(parsedDate, isoDateFormat)
			} catch (error) {
				console.error(`Failed to parse date: ${v}`, error)
				return undefined
			}
		} else {
			return undefined
		}
	}

	let date: string | undefined = $derived(computeDate(value))

	function updateValue(newDate: string | undefined) {
		if (newDate && isValid(new Date(newDate))) {
			try {
				const dateFromValue = new Date(newDate + 'T00:00:00')
				value = format(dateFromValue, isoDateFormat)
				dispatch('change', value)
			} catch (error) {
				console.error('Failed to parse date:', error)
			}
		}
	}

	let randomId = 'datetarget-' + Math.random().toString(36).substring(7)
</script>

<div
	class="flex flex-row gap-1 items-center w-full"
	id={randomId}
	onpointerdown={bubble('pointerdown')}
	onfocus={bubble('focus')}
>
	<!-- svelte-ignore a11y_autofocus -->
	<input
		{disabled}
		type="date"
		value={date}
		{autofocus}
		class="!w-full app-editor-input"
		min={minDate}
		max={maxDate}
		onchange={(e) => {
			const newDate = e.currentTarget.value
			if (newDate) {
				updateValue(newDate)
			} else {
				value = null
				dispatch('change', value)
			}
		}}
	/>
</div>
