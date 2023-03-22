<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { X } from 'lucide-svelte'

	export let value: any = ''
	export let placeholder = ''
	export let type = 'text'
	export let inputClass = ''
	export let wrapperClass = ''
	export let buttonClass = ''
	const dispatch = createEventDispatcher()

	$: isNumeric = type.match(/^(number|range)$/)
	$: dispatch('change', value)

	function handleInput(e) {
		value = isNumeric ? +e.target.value : e.target.value
	}

	function clear() {
		value = isNumeric ? null : ''
	}
</script>

<div class="relative grow {wrapperClass}">
	<input
		{type}
		{value}
		{placeholder}
		class="duration-200 {(type === 'number' && value ? '!pr-7 ' : '') + inputClass}"
		{...$$restProps}
		on:input={handleInput}
		on:focus
		on:blur
	/>
	{#if value}
		<button
			transition:fade|local={{ duration: 100 }}
			class="absolute z-10 top-1.5 right-1 rounded-full p-1 duration-200 hover:bg-gray-200 {buttonClass}"
			aria-label="Clear"
			on:click|preventDefault|stopPropagation={clear}
		>
			<X size={14} />
		</button>
	{/if}
	<slot />
</div>
