<script lang="ts">
	export let value: number
	export let index: number
</script>

<div
	class={value === index
		? 'border-b-2 border-gray-900 text-gray-900 py-1 px-2 cursor-pointer font-bold text-sm'
		: 'py-1 px-2 cursor-pointer	font-medium  text-sm'}
	on:click={() => (value = index)}
>
	<slot />
</div>
