<script lang="ts">
	import { ContextIconMap } from './core'
	import type { ContextElement } from './core'

	export let availableContext: ContextElement[]
	export let selectedContext: ContextElement[]
	export let onSelect: (element: ContextElement) => void
</script>

<div class="flex flex-col gap-1 text-tertiary text-xs p-1 min-w-24 max-h-48 overflow-y-scroll">
	{#if availableContext.filter((c) => !selectedContext.find((sc) => sc.type === c.type && sc.title === c.title)).length === 0}
		<div class="text-center text-tertiary text-xs">No available context</div>
	{:else}
		{#each availableContext as element}
			{#if !selectedContext.find((c) => c.type === element.type && c.title === element.title)}
				<button
					class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal"
					on:click={() => onSelect(element)}
				>
					<svelte:component this={ContextIconMap[element.type]} size={16} />
					{element.title}
				</button>
			{/if}
		{/each}
	{/if}
</div> 