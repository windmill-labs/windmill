<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'

	export let min = 0
	export let max = 100
	export let initialValue = 0
	export let value = typeof initialValue === 'string' ? parseInt(initialValue) : initialValue
	export let disabled: boolean = false
	export let defaultValue: number | undefined = undefined

	let step: number = 1

	let slider: HTMLElement

	function calculateAxisStep(min: number, max: number): number {
		const range = max - min
		return range < 100 ? 1 : range / 20
	}

	$: if (value === null) {
		value = 0
	}

	$: axisStep = calculateAxisStep(min, max)

	const format = (v, i, p) => {
		return `${v}`
	}

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
	$: handleWidth = `${Math.max(max.toString().length ?? 2, 2)}em`
</script>

<div class="flex flex-row w-full mx-2 items-center gap-8">
	<!-- svelte-ignore a11y-no-static-element-interactions -->

	<div
		class={'grow'}
		style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; --handle-width: {handleWidth}; --handle-border: 4px;"
		on:pointerdown|stopPropagation
		on:keydown={handleKeyDown}
	>
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
	</div>

	<input bind:value type="number" class="!w-16 h-8 !text-xs mb-6" {max} {min} {disabled} />
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
