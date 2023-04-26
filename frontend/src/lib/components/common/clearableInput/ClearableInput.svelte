<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { X } from 'lucide-svelte'

	export let value: any = ''
	export let placeholder = ''
	export let type: 'text' | 'textarea' | 'number' = 'text'
	export let inputClass = ''
	export let wrapperClass = ''
	export let buttonClass = ''
	const dispatch = createEventDispatcher()
	let isHovered = false

	$: isNumeric = ['number', 'range'].includes(type)
	$: dispatch('change', value)

	function handleInput(e) {
		value = isNumeric ? +e.target.value : e.target.value
	}

	function clear() {
		value = ''
	}
</script>

<div
	class="relative grow {wrapperClass}"
	on:mouseenter={() => (isHovered = true)}
	on:mouseleave={() => (isHovered = false)}
>
	{#if type === 'textarea'}
		<textarea
			{value}
			{placeholder}
			rows="1"
			class="resize-y duration-200 {inputClass}"
			{...$$restProps}
			on:input={handleInput}
			on:focus
			on:blur
		/>
	{:else}
		<input
			{type}
			{value}
			{placeholder}
			class="duration-200 {(type === 'number' && value ? '!pr-[26px] ' : '') + inputClass}"
			{...$$restProps}
			on:input={handleInput}
			on:focus
			on:blur
		/>
	{/if}
	{#if value && isHovered}
		<button
			transition:fade|local={{ duration: 80 }}
			class="absolute z-10 top-[9.5px] right-2 rounded-full p-0.5 text-white bg-gray-300
			duration-200 hover:bg-gray-600 focus:bg-gray-600 {buttonClass}"
			aria-label="Clear"
			on:click|preventDefault|stopPropagation={clear}
		>
			<X size={11} class="" />
		</button>
	{/if}
	<slot />
</div>
