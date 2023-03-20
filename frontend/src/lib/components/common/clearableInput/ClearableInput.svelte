<script>
	import { X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'

	export let placeholder = ''
	export let value = undefined
	export let type = 'text'
	export let inputClass = ''
	export let wrapperClass = ''

	$: isNumeric = type.match(/^(number|range)$/)

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
		class={inputClass}
		{...$$restProps}
		on:input={handleInput}
		on:focus
		on:blur
	/>
	{#if value}
		<button
			transition:fade|local={{ duration: 100 }}
			class="absolute z-10 top-1.5 right-1 rounded-full p-1 duration-200 hover:bg-gray-200"
			aria-label="Remove styles"
			on:click|preventDefault|stopPropagation={clear}
		>
			<X size={14} />
		</button>
	{/if}
</div>
