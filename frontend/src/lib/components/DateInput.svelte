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
		dateFormat = 'dd-MM-yyyy',
		disabled = false
	}: Props = $props()

	const defaultDateFormat = 'dd-MM-yyyy'
	const defaultHtmlDateFormat = 'yyyy-MM-dd'

	// Ensure we always have a valid format (prop can be undefined or empty string)
	function getFormat() {
		return dateFormat && dateFormat.length > 0 ? dateFormat : defaultDateFormat
	}

	const dispatch = createEventDispatcher()

	function computeDate(v: string | null | undefined, formatStr: string) {
		if (v && v.length > 0) {
			try {
				let parsedDate = parse(v, formatStr, new Date())
				if (parsedDate.toString() === 'Invalid Date') {
					console.debug('falling back to default html date format')
					parsedDate = parse(v, defaultHtmlDateFormat, new Date())
				}
				return format(parsedDate, defaultHtmlDateFormat)
			} catch (error) {
				console.error(`Failed to parse date: ${v} with format ${formatStr}`, error)
				return undefined
			}
		} else {
			return undefined
		}
	}

	let date: string | undefined = $derived(computeDate(value, getFormat()))

	function updateValue(newDate: string | undefined) {
		if (newDate && isValid(new Date(newDate))) {
			try {
				const dateFromValue = new Date(newDate + 'T00:00:00')
				const parsedDate = format(dateFromValue, getFormat())
				value = parsedDate
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
