<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { format, isValid, parse } from 'date-fns'

	export let value: string | undefined = undefined
	export let autofocus: boolean | null = false
	export let minDate: string | undefined = undefined
	export let maxDate: string | undefined = undefined
	export let dateFormat: string | undefined = 'dd-MM-yyyy'

	let date: string | undefined = computeDate(value)

	const dispatch = createEventDispatcher()

	function computeDate(value: string | undefined) {
		if (dateFormat === undefined) {
			dateFormat = 'dd-MM-yyyy'
		}
		if (value) {
			const res = format(parse(value, dateFormat, new Date()), 'yyyy-MM-dd')
			return res
		} else {
			return undefined
		}
	}

	function updateValue(newDate: string | undefined) {
		if (newDate && isValid(new Date(newDate))) {
			if (dateFormat === undefined) {
				dateFormat = 'dd-MM-yyyy'
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

<div class="flex flex-row gap-1 items-center w-full" id={randomId} on:pointerdown on:focus>
	<input
		type="date"
		bind:value={date}
		{autofocus}
		class="!w-full app-editor-input"
		min={minDate}
		max={maxDate}
		on:change={() => {
			if (date) {
				updateValue(date)
			}
		}}
	/>
</div>
