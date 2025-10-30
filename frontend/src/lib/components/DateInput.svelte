<script lang="ts">
	import { createBubbler } from 'svelte/legacy';

	const bubble = createBubbler();
	import { createEventDispatcher } from 'svelte'
	import { format, isValid, parse } from 'date-fns'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		value?: string | undefined;
		autofocus?: boolean | null;
		minDate?: string | undefined;
		maxDate?: string | undefined;
		dateFormat?: string | undefined;
		disabled?: boolean;
	}

	let {
		value = $bindable(undefined),
		autofocus = false,
		minDate = undefined,
		maxDate = undefined,
		dateFormat = $bindable('dd-MM-yyyy'),
		disabled = false
	}: Props = $props();

	const defaultDateFormat = 'dd-MM-yyyy'
	const defaultHtmlDateFormat = 'yyyy-MM-dd'

	let date: string | undefined = $state(computeDate(value))

	const dispatch = createEventDispatcher()

	function computeDate(value: string | undefined) {
		if (dateFormat === undefined) {
			dateFormat = defaultDateFormat
		}
		if (value && value.length > 0) {
			try {
				let date = parse(value, dateFormat, new Date())
				if (date.toString() === 'Invalid Date') {
					console.debug('falling back to default html date format')
					date = parse(value, defaultHtmlDateFormat, new Date())
				}
				const res = format(date, defaultHtmlDateFormat)
				return res
			} catch (error) {
				sendUserToast(
					`Failed to parse date: ${value} with format ${dateFormat} and ${defaultHtmlDateFormat}`,
					true
				)
				console.error(`Failed to parse date: ${value}`, error)
				return undefined
			}
		} else {
			return undefined
		}
	}

	function updateValue(newDate: string | undefined) {
		if (newDate && isValid(new Date(newDate))) {
			if (dateFormat === undefined) {
				dateFormat = defaultDateFormat
			}

			try {
				let dateFromValue: Date | undefined = newDate ? new Date(newDate + 'T00:00:00') : undefined

				if (dateFromValue === undefined) {
					return
				}

				const parsedDate = format(dateFromValue, dateFormat)
				value = parsedDate

				dispatch('change', value)
			} catch (error) {
				console.error('Failed to parse date:', error)
				return
			}
		}
	}

	let randomId = 'datetarget-' + Math.random().toString(36).substring(7)
</script>

<div class="flex flex-row gap-1 items-center w-full" id={randomId} onpointerdown={bubble('pointerdown')} onfocus={bubble('focus')}>
	<!-- svelte-ignore a11y_autofocus -->
	<input
		{disabled}
		type="date"
		bind:value={date}
		{autofocus}
		class="!w-full app-editor-input"
		min={minDate}
		max={maxDate}
		onchange={() => {
			if (date) {
				updateValue(date)
			}
		}}
	/>
</div>
