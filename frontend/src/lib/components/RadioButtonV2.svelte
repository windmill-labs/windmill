<script lang="ts">
	export let options: [{ title: string; desc?: string }, string][]
	export let value: any

	import { createEventDispatcher } from 'svelte'

	$: dispatch('change', value)

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-row space-x-4 m-4">
	{#each options as [label, val]}
		<button
			type="button"
			on:click={() => {
				value = val
			}}
			class="default-secondary-button-v2 mb-2 w-full"
			class:selected={value == val}
		>
			<h3>{label.title}</h3>
			<p>
				{label.desc}
			</p>
		</button>
	{/each}
</div>

<style>
	.selected {
		@apply bg-blue-500/90 text-white;
	}
</style>
