<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createEventDispatcher, untrack } from 'svelte'
	import { Button } from './common'
	import { Clock, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import TextInput from './text_input/TextInput.svelte'
	// import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'

	interface Props {
		// import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
		value?: string | null | undefined
		clearable?: boolean
		autofocus?: boolean | null
		useDropdown?: boolean
		minDate?: string | undefined
		maxDate?: string | undefined
		disabled?: boolean | undefined
		inputClass?: string | undefined
		/**
		 * 'naive' will ignore timezone and return a date without timezone
		 * 'local' will use the local timezone of the user
		 */
		timezone?: 'naive' | 'local'
	}

	let {
		value = $bindable(undefined),
		clearable = false,
		autofocus = false,
		useDropdown = false,
		minDate = undefined,
		maxDate = undefined,
		disabled = undefined,
		inputClass = undefined,
		timezone = 'local'
	}: Props = $props()

	let date: string | undefined = $state(undefined)
	let time: string | undefined = $state(undefined)

	// let format: 'local' | 'utc' = 'local'

	function parseValue(value: string | null | undefined = undefined) {
		let dateFromValue: Date | undefined = value ? new Date(value) : undefined
		if (!isValidDate(dateFromValue)) {
			date = undefined
			time = '12:00'
			return
		}

		let year = timezone === 'local' ? dateFromValue?.getFullYear() : dateFromValue?.getUTCFullYear()
		let month = timezone === 'local' ? dateFromValue?.getMonth() : dateFromValue?.getUTCMonth()
		let day = timezone === 'local' ? dateFromValue?.getDate() : dateFromValue?.getUTCDate()
		let hours = timezone === 'local' ? dateFromValue.getHours() : dateFromValue.getUTCHours()
		let minutes = timezone === 'local' ? dateFromValue.getMinutes() : dateFromValue.getUTCMinutes()

		date = `${year.toString()}-${(month + 1)
			.toString()
			.padStart(2, '0')}-${day.toString().padStart(2, '0')}`

		time = `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`
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
		// Handle cleared date - if date is empty string (user cleared) but value still has a value
		if (date === '' && value) {
			value = null
			dispatchIfMounted('change', value)
			return
		}

		if (date && time && (initialDate != date || initialTime != time)) {
			let newDate = new Date(timezone === 'local' ? `${date}T${time}` : `${date}T${time}Z`)
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

	function isValidDate(d: Date | undefined): d is Date {
		return d instanceof Date && !isNaN(d as any)
	}

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	function setTimeLater(mins: number) {
		let newDate = new Date()
		if (timezone === 'naive') {
			newDate.setMinutes(newDate.getMinutes() - newDate.getTimezoneOffset())
		}
		newDate.setMinutes(newDate.getMinutes() + mins)
		value = newDate.toISOString()
		dispatch('change', value)
	}

	let randomId = 'datetarget-' + Math.random().toString(36).substring(7)
</script>

<div
	class="flex flex-row gap-1 items-center w-full relative"
	id={randomId}
	onpointerdown={bubble('pointerdown')}
	onfocus={bubble('focus')}
>
	<!-- svelte-ignore a11y_autofocus -->
	<TextInput
		inputProps={{ type: 'date', autofocus, disabled, min: minDate, max: maxDate }}
		bind:value={date}
		class={twMerge('!w-3/4 ', inputClass)}
	/>
	<TextInput
		inputProps={{ type: 'time', disabled }}
		bind:value={time}
		class={twMerge('!w-1/4 min-w-[100px] !max-h-full', inputClass)}
	/>
	<Button
		variant="default"
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
			variant="default"
			unifiedSize="md"
			wrapperClasses="h-full"
			{disabled}
			iconOnly
			endIcon={{ icon: X }}
			on:click={() => {
				value = null
				dispatch('clear')
			}}
		></Button>
	{/if}
	<!-- <div>
		<ToggleButtonGroup bind:selected={format} let:item>
			<ToggleButton light small value={'local'} label="local" {item} />
			<ToggleButton light small value={'utc'} label="utc" {item} />
		</ToggleButtonGroup>
	</div> -->
</div>
