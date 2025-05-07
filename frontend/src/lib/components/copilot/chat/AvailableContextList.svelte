<script lang="ts">
	import { ContextIconMap } from './core'
	import type { ContextElement } from './core'

	export let availableContext: ContextElement[]
	export let selectedContext: ContextElement[]
	export let onSelect: (element: ContextElement) => void
	export let showAllAvailable = false
	export let stringSearch = ''
	export let selectedIndex = 0

	$: actualAvailableContext = showAllAvailable
		? availableContext.filter(
				(c) => !stringSearch || c.title.toLowerCase().includes(stringSearch.toLowerCase())
		  )
		: availableContext.filter(
				(c) =>
					!selectedContext.find((sc) => sc.type === c.type && sc.title === c.title) &&
					(!stringSearch || c.title.toLowerCase().includes(stringSearch.toLowerCase()))
		  )
</script>

<div class="flex flex-col gap-1 text-tertiary text-xs p-1 min-w-24 max-h-48 overflow-y-scroll">
	{#if actualAvailableContext.length === 0}
		<div class="text-center text-tertiary text-xs">No available context</div>
	{:else}
		{#each actualAvailableContext as element, i}
			<button
				class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal {i ===
				selectedIndex
					? 'bg-surface-hover'
					: ''}"
				on:click={() => onSelect(element)}
			>
				<svelte:component this={ContextIconMap[element.type]} size={16} />
				{element.title}
			</button>
		{/each}
	{/if}
</div>
