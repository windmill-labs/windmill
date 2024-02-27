<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button } from './common'

	export let value: string | undefined = undefined

	export let autofocus: boolean | null = false
	export let useDropdown: boolean = false
	export let minDate: string | undefined = undefined
	export let maxDate: string | undefined = undefined

	let date: string | undefined = undefined
	let time: string | undefined = undefined

	function parseValue(value: string | undefined = undefined) {
		let dateFromValue: Date | undefined = value ? new Date(value) : undefined

		date = isValidDate(dateFromValue) ? dateFromValue!.toISOString().split('T')[0] : undefined
		time = isValidDate(dateFromValue)
			? `${dateFromValue!.getHours().toString().padStart(2, '0')}:${dateFromValue!
					.getMinutes()
					.toString()
					.padStart(2, '0')}`
			: '12:00'
	}

	$: parseValue(value)

	let initialDate = date
	let initialTime = time

	function parseDateAndTime(date: string | undefined, time: string | undefined) {
		if (date && time && (initialDate != date || initialTime != time)) {
			let newDate = new Date(`${date}T${time}`)
			value = newDate.toISOString()
			dispatch('change', value)
		}
	}

	$: parseDateAndTime(date, time)

	function isValidDate(d: Date | undefined): boolean {
		return d instanceof Date && !isNaN(d as any)
	}

	const dispatch = createEventDispatcher()

	function setTimeLater(mins: number) {
		let newDate = new Date()
		newDate.setMinutes(newDate.getMinutes() + mins)
		value = newDate.toISOString()
		dispatch('change', value)
	}

	let randomId = 'datetarget-' + Math.random().toString(36).substring(7)
</script>

<div class="flex flex-row gap-1 items-center w-full" id={randomId} on:pointerdown on:focus>
	<!-- svelte-ignore a11y-autofocus -->
	<input type="date" bind:value={date} {autofocus} class="!w-3/4" min={minDate} max={maxDate} />
	<input type="time" bind:value={time} class="!w-1/4 min-w-[100px]" />
	<Button
		variant="border"
		color="light"
		size="xs"
		portalTarget={`#${randomId}`}
		dropdownItems={useDropdown
			? [
					{
						label: 'In 15 minutes',
						onClick: () => {
							setTimeLater(15)
						}
					},
					{
						label: 'In 1 hour',
						onClick: () => {
							setTimeLater(60)
						}
					},
					{
						label: 'Tomorrow',
						onClick: () => {
							setTimeLater(60 * 24)
						}
					},
					{
						label: 'In 1 week',
						onClick: () => {
							setTimeLater(7 * 60 * 24)
						}
					}
			  ]
			: undefined}
		on:click={() => {
			setTimeLater(0)
		}}>Now</Button
	>
</div>
