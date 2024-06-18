<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'

	export let min = 0
	export let max = 100
	export let initialValue = 0
	export let value = typeof initialValue === 'string' ? parseInt(initialValue) : initialValue
	export let disabled: boolean = false

	let step: number = 1

	let slider: HTMLElement

	function calculateAxisStep(min: number, max: number): number {
		const range = max - min
		return range < 100 ? 1 : range / 20
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
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={'grow'}
	style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; "
	on:pointerdown|stopPropagation
	on:keydown={handleKeyDown}
>
	<RangeSlider
		id="range-slider"
		springValues={{ stiffness: 1, damping: 1 }}
		bind:slider
		min={min == undefined ? 0 : +min}
		max={max == undefined ? 1 : +max}
		on:change={(e) => {
			value = e.detail.value
		}}
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

<style>
	:global(#range-slider.rangeSlider) {
		font-size: 12px;
		text-transform: uppercase;
	}

	:global(.dark #range-slider.rangeSlider) {
		background-color: #3b4252;
	}

	:global(#range-slider.rangeSlider .rangeHandle) {
		width: 3em !important;
		height: 2em;
		display: flex;
		justify-items: center;
		justify-content: center;
	}

	:global(#range-slider.rangeSlider .rangeFloat) {
		opacity: 1;
		background: transparent;
		top: 50%;
		transform: translate(-50%, -50%);
	}

	:global(.dark #range-slider.rangeSlider > .rangePips > .pip) {
		color: #eeeeee;
	}
</style>
