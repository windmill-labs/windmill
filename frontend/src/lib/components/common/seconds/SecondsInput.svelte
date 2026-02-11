<script lang="ts">
	import { untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from '../../text_input/TextInput.svelte'
	import type { ButtonType } from '../button/model'
	import CloseButton from '../CloseButton.svelte'

	const ONE_DAY_IN_SECONDS = 86400 as const
	const ONE_HOUR_IN_SECONDS = 3600 as const
	const ONE_MINUTE_IN_SECONDS = 60 as const

	const segmentClass =
		'no-default-style !bg-transparent border-none !w-7 text-right p-0 text-xs font-medium focus:ring-0 focus:outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none'

	interface Props {
		seconds?: number
		disabled?: boolean
		max?: number | undefined
		size?: ButtonType.UnifiedSize
		clearable?: boolean
		defaultValue?: number | undefined
		onfocus?: (e: FocusEvent) => void
	}

	let {
		seconds = $bindable(),
		disabled = false,
		max = undefined,
		size = 'md',
		clearable = false,
		defaultValue = undefined,
		onfocus
	}: Props = $props()

	let day: number = $state(0)
	let hour: number = $state(0)
	let min: number = $state(0)
	let sec: number = $state(0)

	let containerFocused = $state(false)
	let hasValue = $derived(seconds != null)

	function convertSecondsToTime(s: number) {
		day = Math.floor(s / ONE_DAY_IN_SECONDS)
		s -= day * ONE_DAY_IN_SECONDS

		hour = Math.floor(s / ONE_HOUR_IN_SECONDS)
		s -= hour * ONE_HOUR_IN_SECONDS

		min = Math.floor(s / ONE_MINUTE_IN_SECONDS)
		s -= min * ONE_MINUTE_IN_SECONDS

		sec = s
	}

	function convertUnitsToSeconds() {
		seconds =
			(day || 0) * ONE_DAY_IN_SECONDS +
			(hour || 0) * ONE_HOUR_IN_SECONDS +
			(min || 0) * ONE_MINUTE_IN_SECONDS +
			(sec || 0)
		if (seconds < 0) seconds = 0
		if (max && seconds > max) {
			seconds = max
		}
	}

	function clamp(value: number, maxVal: number) {
		return Math.max(0, Math.min(maxVal, value))
	}

	function handleKeydown(
		e: KeyboardEvent,
		getter: () => number,
		setter: (v: number) => void,
		maxVal: number
	) {
		if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
			e.preventDefault()
			setter(clamp((getter() || 0) + (e.key === 'ArrowUp' ? 1 : -1), maxVal))
			convertUnitsToSeconds()
		}
	}

	function handleInput(e: Event, setter: (v: number) => void, maxVal: number) {
		const input = e.currentTarget as HTMLInputElement
		const clamped = clamp(parseInt(input.value) || 0, maxVal)
		setter(clamped)
		input.value = String(clamped)
		convertUnitsToSeconds()
	}

	function handleFocus(e: FocusEvent) {
		containerFocused = true
		if (seconds == null) {
			seconds = 0
		}
		onfocus?.(e)
	}

	function handleBlur() {
		containerFocused = false
	}

	$effect(() => {
		const s = seconds
		untrack(() => convertSecondsToTime(s ?? 0))
	})
</script>

<div class="flex flex-col gap-1">
	<div class="flex items-center gap-1">
		<div
			class={twMerge(
				inputBaseClass,
				inputSizeClasses[size],
				inputBorderClass({ forceFocus: containerFocused }),
				'flex items-center gap-0.5 !w-fit'
			)}
		>
			<div class="flex items-baseline">
				<input
					type="number"
					class={segmentClass}
					value={hasValue ? day || 0 : ''}
					placeholder="_ _"
					min="0"
					{disabled}
					oninput={(e) => handleInput(e, (v) => (day = v), Infinity)}
					onkeydown={(e) =>
						handleKeydown(
							e,
							() => day,
							(v) => (day = v),
							Infinity
						)}
					onfocus={handleFocus}
					onblur={handleBlur}
				/>
				<span class="text-secondary text-2xs inline-grid [&>*]:col-start-1 [&>*]:row-start-1"
					><span class="invisible">days</span><span>{day && day > 1 ? 'days' : 'day'}</span></span
				>
			</div>
			<div class="flex items-baseline">
				<input
					type="number"
					class={segmentClass}
					value={hasValue ? hour || 0 : ''}
					placeholder="_ _"
					min="0"
					max="23"
					{disabled}
					oninput={(e) => handleInput(e, (v) => (hour = v), 23)}
					onkeydown={(e) =>
						handleKeydown(
							e,
							() => hour,
							(v) => (hour = v),
							23
						)}
					onfocus={handleFocus}
					onblur={handleBlur}
				/>
				<span class="text-secondary text-2xs inline-grid [&>*]:col-start-1 [&>*]:row-start-1"
					><span class="invisible">hrs</span><span>{hour && hour > 1 ? 'hrs' : 'hr'}</span></span
				>
			</div>
			<div class="flex items-baseline">
				<input
					type="number"
					class={segmentClass}
					value={hasValue ? min || 0 : ''}
					placeholder="_ _"
					min="0"
					max="59"
					{disabled}
					oninput={(e) => handleInput(e, (v) => (min = v), 59)}
					onkeydown={(e) =>
						handleKeydown(
							e,
							() => min,
							(v) => (min = v),
							59
						)}
					onfocus={handleFocus}
					onblur={handleBlur}
				/>
				<span class="text-secondary text-2xs inline-grid [&>*]:col-start-1 [&>*]:row-start-1"
					><span class="invisible">mins</span><span>{min && min > 1 ? 'mins' : 'min'}</span></span
				>
			</div>
			<div class="flex items-baseline">
				<input
					type="number"
					class={segmentClass}
					value={hasValue ? sec || 0 : ''}
					placeholder="_ _"
					min="0"
					max="59"
					{disabled}
					oninput={(e) => handleInput(e, (v) => (sec = v), 59)}
					onkeydown={(e) =>
						handleKeydown(
							e,
							() => sec,
							(v) => (sec = v),
							59
						)}
					onfocus={handleFocus}
					onblur={handleBlur}
				/>
				<span class="text-secondary text-2xs inline-grid [&>*]:col-start-1 [&>*]:row-start-1"
					><span class="invisible">secs</span><span>{sec && sec > 1 ? 'secs' : 'sec'}</span></span
				>
			</div>
		</div>
		{#if clearable && !disabled && seconds != null && seconds !== defaultValue}
			<CloseButton
				class="bg-transparent text-secondary hover:text-primary"
				noBg
				small
				on:close={() => {
					seconds = defaultValue
				}}
			/>
		{/if}
	</div>
	{#if seconds != null}
		<span class="text-2xs text-hint">
			= {seconds.toLocaleString()} second{seconds === 1 ? '' : 's'}
		</span>
	{:else}
		<span class="text-2xs text-hint"> no time set </span>
	{/if}
</div>
