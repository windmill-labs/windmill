<script lang="ts">
	import ColorPicker, { ChromeVariant } from 'svelte-awesome-color-picker'
	import type { StaticInput } from '../../../inputType'

	export let componentInput: StaticInput<string>
	let isOpen = false
</script>

<div class="color-picker-input">
	<input
		type="text"
		readonly
		value={componentInput.value}
		on:focus|preventDefault|stopPropagation={() => (isOpen = true)}
	/>
	<ColorPicker
		bind:isOpen
		bind:hex={componentInput.value}
		label={componentInput.value}
		canChangeMode={false}
		components={ChromeVariant}
		toRight
		--focus-color="#e0e7ff"
	/>
</div>

<style global>
	.color-picker-input span > label.svelte-rogbpz {
		display: none;
	}

	.color-picker-input span > div.wrapper {
		box-shadow: 0 10px 40px -5px rgba(0, 0, 0, 0.25) !important;
		border-style: none !important;
		border-radius: 0.375rem !important;
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
</style>
