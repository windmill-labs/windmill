<script lang="ts">
	export let options: [{ title: string; desc?: string }, string][]
	export let value: any

	import { createEventDispatcher } from 'svelte'

	$: dispatch('change', value)

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-row space-x-4 m-4 reveal">
	{#each options as [label, val]}
		<button
			type="button"
			on:click={() => {
				value = val
			}}
			class="flex flex-col default-secondary-button-v2 mb-2 w-full"
			class:selected={value == val}
		>
			<h2 class="mb-2">{label.title}</h2>
			<p class="hidden">
				{label.desc}
			</p>
		</button>
	{/each}
</div>

<style>
	.reveal:hover > button > p {
		display: block;
	}
	.selected {
		@apply bg-gray-300/90;
	}
</style>
