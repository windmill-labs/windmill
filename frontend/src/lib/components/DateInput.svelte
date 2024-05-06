<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let value: string | undefined = undefined
	export let autofocus: boolean | null = false
	export let minDate: string | undefined = undefined
	export let maxDate: string | undefined = undefined

	let date: string | undefined = undefined

	function parseValue(value: string | undefined = undefined) {
		let dateFromValue: Date | undefined = value ? new Date(value) : undefined
		date = isValidDate(dateFromValue)
			? `${dateFromValue!.getFullYear().toString()}-${(dateFromValue!.getMonth() + 1)
					.toString()
					.padStart(2, '0')}-${dateFromValue!.getDate().toString().padStart(2, '0')}`
			: undefined
	}

	$: parseValue(value)

	function isValidDate(d: Date | undefined): boolean {
		return d instanceof Date && !isNaN(d as any)
	}

	const dispatch = createEventDispatcher()

	function setDate(newDate: string | undefined) {
		if (newDate && isValidDate(new Date(newDate))) {
			let parsedDate = new Date(newDate)
			value = parsedDate.toISOString()
			dispatch('change', value)
		}
	}

	$: setDate(date)

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
	/>
</div>
