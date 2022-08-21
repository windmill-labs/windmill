<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let title: string | undefined = undefined
	export let clickable: boolean = false
	const dispatch = createEventDispatcher()
</script>

<div
	on:click={() => dispatch('click')}
	class="flex items-center justify-between flex-wrap px-6 py-2"
	class:cursor-pointer={clickable}
>
	{#if title}
		<h3 class="text-sm font-bold text-gray-900">{title}</h3>
	{/if}
	<slot />
</div>
