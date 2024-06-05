<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'

	export let min = 0
	export let max = 100
	export let initialValue = 0
	export let value = typeof initialValue === 'string' ? parseInt(initialValue) : initialValue
	export let disabled: boolean = false
	export let step: number = 1
	export let axisStep: number = 1

	let slider: HTMLElement

	const format = (v, i, p) => {
		return `${v}`
	}
</script>

<div
	class={'grow'}
	style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; "
	on:pointerdown|stopPropagation
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
		width: 2em;
		height: 2em;
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
