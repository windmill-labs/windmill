<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { X } from 'lucide-svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	export let value: any = ''
	export let placeholder = ''
	export let type: 'text' | 'textarea' | 'number' = 'text'
	export let inputClass = ''
	export let wrapperClass = ''
	export let buttonClass = ''
	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)
	let isHovered = false

	$: isNumeric = ['number', 'range'].includes(type)
	$: dispatchIfMounted('change', value)

	function handleInput(e) {
		value = isNumeric ? +e.target.value : e.target.value
	}

	function clear() {
		value = ''
	}

	$: if (value === undefined) value = ''
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
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
			class="resize-y {inputClass}"
			{...$$restProps}
			on:input={handleInput}
			on:keydown|stopPropagation
			on:focus
			on:blur
		></textarea>
	{:else}
		<input
			{type}
			{value}
			{placeholder}
			class=" {(value ? '!pr-[26px] ' : '') + inputClass}"
			{...$$restProps}
			on:input={handleInput}
			on:keydown|stopPropagation
			on:focus
			on:blur
		/>
	{/if}
	{#if value && isHovered}
		<button
			transition:fade|local={{ duration: 80 }}
			class="absolute z-10 top-[9.5px] right-2 rounded-full p-0.5 text-primary bg-surface-secondary
			 hover:bg-surface-hover focus:bg-surface-hover {buttonClass}"
			aria-label="Clear"
			on:click|preventDefault|stopPropagation={clear}
		>
			<X size={11} class="" />
		</button>
	{/if}
	<slot />
</div>
