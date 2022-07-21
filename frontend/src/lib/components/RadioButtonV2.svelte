<script lang="ts">
	export let options: [{ title: string; desc?: string }, string][]
	export let value: any

	import { createEventDispatcher } from 'svelte'
	import Tooltip from './Tooltip.svelte'

	$: dispatch('change', value)

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-row gap-x-4 m-4 reveal flex-wrap ">
	{#each options as [label, val]}
		<button
			type="button"
			on:click={() => {
				value = val
			}}
			class="flex flex-col default-secondary-button-v2 mb-2 grow"
			class:selected={value == val}
		>
			<h2 class="mb-2 whitespace-nowrap">{label.title} <Tooltip>{label.desc}</Tooltip></h2>
		</button>
	{/each}
</div>

<style>
	.selected {
		@apply bg-blue-500/70 text-white;
	}
	.selected > h2 {
		@apply text-white;
	}
</style>
