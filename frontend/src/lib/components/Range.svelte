<script lang="ts">
	import { run, createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import RangeSlider from 'svelte-range-slider-pips'

	interface Props {
		min?: number
		max?: number
		initialValue?: number
		value?: any
		disabled?: boolean
		defaultValue?: number | undefined
		format?: (value: number) => string
		hideInput?: boolean
	}

	let {
		min = 0,
		max = 100,
		initialValue = 0,
		value = $bindable(typeof initialValue === 'string' ? parseInt(initialValue) : initialValue),
		disabled = false,
		defaultValue = undefined,
		format = (v) => `${v}`,
		hideInput = false
	}: Props = $props()

	let step: number = 1

	let slider: HTMLElement | undefined = $state()

	function calculateAxisStep(min: number, max: number): number {
		const range = max - min
		return range < 100 ? 1 : range / 20
	}

	run(() => {
		if (value === null) {
			value = 0
		}
	})

	let axisStep = $derived(calculateAxisStep(min, max))

	function handleKeyDown(event: KeyboardEvent) {
		if (disabled) return

		switch (event.key) {
			case 'ArrowLeft':
				if (value > min) {
					value = Math.max(value - step, min)
				}
				break
			case 'ArrowRight':
				if (value < max) {
					value = Math.min(value + step, max)
				}
				break
		}
		event.preventDefault()
	}

	// Calculate the handle width based on the length of the max value
	let handleWidth = $derived(`${Math.max(max.toString().length ?? 2, 2)}em`)
</script>

<div class="flex flex-row w-full mx-2 items-center gap-8">
	<!-- svelte-ignore a11y_no_static_element_interactions -->

	<div
		class={'grow'}
		style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; --handle-width: {handleWidth}; --handle-border: 4px;"
		onpointerdown={stopPropagation(bubble('pointerdown'))}
		onkeydown={handleKeyDown}
		>{#if max <= min}
			<div class="text-secondary text-sm"
				>Impossible to display range: {`max (${max}) <= min (${min})`}</div
			>
		{:else}
			<RangeSlider
				id="range-slider-form"
				springValues={{ stiffness: 1, damping: 1 }}
				bind:slider
				min={min == undefined ? 0 : +min}
				max={max == undefined ? 1 : +max}
				on:change={(e) => {
					value = e.detail.value
				}}
				{defaultValue}
				{disabled}
				values={[value]}
				pips
				float
				first="label"
				last="label"
				step={step ?? 1}
				pipstep={(axisStep ?? 1) / (step ?? 1)}
				formatter={format}
			/>
		{/if}
	</div>

	{#if !hideInput}
		<input bind:value type="number" class="!w-16 h-8 !text-xs mb-6" {max} {min} {disabled} />
	{/if}
</div>

<style>
	:global(#range-slider-form.rangeSlider) {
		font-size: 12px;
		text-transform: uppercase;
	}

	:global(.dark #range-slider-form.rangeSlider) {
		background-color: #3b4252;
	}

	:global(#range-slider-form.rangeSlider .rangeHandle) {
		width: var(--handle-width, 2em) !important;
		height: 2em;
		display: flex;
		justify-items: center;
		justify-content: center;
	}

	:global(#range-slider-form.rangeSlider .rangeFloat) {
		opacity: 1;
		background: transparent;
		top: 50%;
		transform: translate(-50%, -50%);
	}

	:global(.dark #range-slider-form.rangeSlider > .rangePips > .pip) {
		color: #eeeeee;
	}
</style>
