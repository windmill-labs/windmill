<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import ColorPicker, { ChromeVariant } from 'svelte-awesome-color-picker'
	import { ClearableInput } from '../../../../common'
	import { createPopperActions } from 'svelte-popperjs'
	import { fade } from 'svelte/transition'

	export let value: string = '#fff'
	const dispatch = createEventDispatcher()
	const [popperRef, popperContent] = createPopperActions()
	let isOpen = false
	let width: number

	$: dispatch('change', value)

	function open() {
		isOpen = true
	}
</script>

<div use:popperRef bind:clientWidth={width} class="grow">
	<ClearableInput readonly bind:value on:focus={open} />
</div>
{#if isOpen}
	<div
		transition:fade={{ duration: 150 }}
		use:popperContent={{ placement: 'bottom', strategy: 'fixed' }}
		class="color-picker-input z-[1002]"
		style="width: {width > 280 ? width : 280}px;"
	>
		<ColorPicker
			bind:isOpen
			bind:hex={value}
			label={value}
			components={ChromeVariant}
			sliderDirection="horizontal"
			--focus-color="#e0e7ff"
		/>
	</div>
{/if}

<style global>
	.color-picker-input span > label {
		display: none;
	}

	.color-picker-input span > div.wrapper {
		box-shadow: 0 10px 40px -5px rgba(0, 0, 0, 0.25) !important;
		border-style: none !important;
		border-radius: 0.375rem !important;
		top: 6px !important;
		z-index: 30 !important;
	}

	.color-picker-input .slider-wrapper {
		overflow: visible !important;
	}

	.color-picker-input .slider-indicator {
		width: 14px !important;
		height: 14px !important;
		border-radius: 7px !important;
		margin-top: 0 !important;
		background-color: #fafafa !important;
		border: 1px solid #2e2e2eaf !important;
		top: -2px !important;
		box-shadow: 0 2px 6px -2px rgba(0, 0, 0, 0.5) !important;
	}

	.color-picker-input .input-container > input {
		background-color: #ffffff;
		border: 1px solid #d1d5db;
		border-radius: 6px;
	}
</style>
