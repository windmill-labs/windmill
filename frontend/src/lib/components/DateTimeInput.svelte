<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createEventDispatcher, untrack } from 'svelte'
	import { Button } from './common'
	import { Clock, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import { inputBaseClass, inputBorderClass } from './text_input/TextInput.svelte'
	// import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'

	interface Props {
		// import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
		value?: string | undefined
		clearable?: boolean
		autofocus?: boolean | null
		useDropdown?: boolean
		minDate?: string | undefined
		maxDate?: string | undefined
		disabled?: boolean | undefined
		inputClass?: string | undefined
	}

	let {
		value = $bindable(undefined),
		clearable = false,
		autofocus = false,
		useDropdown = false,
		minDate = undefined,
		maxDate = undefined,
		disabled = undefined,
		inputClass = undefined
	}: Props = $props()

	let date: string | undefined = $state(undefined)
	let time: string | undefined = $state(undefined)

	// let format: 'local' | 'utc' = 'local'

	function parseValue(value: string | undefined = undefined) {
		let dateFromValue: Date | undefined = value ? new Date(value) : undefined
		date = isValidDate(dateFromValue)
			? `${dateFromValue!.getFullYear().toString()}-${(dateFromValue!.getMonth() + 1)
					.toString()
					.padStart(2, '0')}-${dateFromValue!.getDate().toString().padStart(2, '0')}`
			: undefined

		time = isValidDate(dateFromValue)
			? `${dateFromValue!.getHours().toString().padStart(2, '0')}:${dateFromValue!
					.getMinutes()
					.toString()
					.padStart(2, '0')}`
			: '12:00'
	}

	$effect(() => {
		value
		untrack(() => {
			parseValue(value)
		})
	})

	let initialDate = untrack(() => date)
	let initialTime = untrack(() => time)

	function parseDateAndTime(date: string | undefined, time: string | undefined) {
		if (date && time && (initialDate != date || initialTime != time)) {
			let newDate = new Date(`${date}T${time}`)
			if (newDate.toString() === 'Invalid Date') return
			if (newDate.getFullYear() < 1900) return

			value = newDate.toISOString()
			dispatchIfMounted('change', value)
		}
	}

	$effect(() => {
		;[date, time]
		untrack(() => {
			parseDateAndTime(date, time)
		})
	})

	function isValidDate(d: Date | undefined): boolean {
		return d instanceof Date && !isNaN(d as any)
	}

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	function setTimeLater(mins: number) {
		let newDate = new Date()
		newDate.setMinutes(newDate.getMinutes() + mins)
		value = newDate.toISOString()
		dispatch('change', value)
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
		type="date"
		bind:value={date}
		{autofocus}
		{disabled}
		class={twMerge(inputBaseClass, inputBorderClass(), 'text-sm !w-3/4 ', inputClass)}
		min={minDate}
		max={maxDate}
	/>
	<input
		type="time"
		bind:value={time}
		class={twMerge(inputBaseClass, inputBorderClass(), 'text-sm !w-1/4 min-w-[100px] ', inputClass)}
		{disabled}
	/>
	<Button
		variant="contained"
		color="light"
		wrapperClasses="h-full"
		btnClasses="bg-surface-secondary"
		startIcon={{
			icon: Clock
		}}
		{disabled}
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
		}}
	>
		Now
	</Button>
	{#if clearable}
		<Button
			variant="border"
			color="light"
			wrapperClasses="h-full"
			{disabled}
			on:click={() => {
				value = undefined
				dispatch('clear')
			}}
		>
			<X size={14} />
		</Button>
	{/if}
	<!-- <div>
		<ToggleButtonGroup bind:selected={format} let:item>
			<ToggleButton light small value={'local'} label="local" {item} />
			<ToggleButton light small value={'utc'} label="utc" {item} />
		</ToggleButtonGroup>
	</div> -->
</div>
