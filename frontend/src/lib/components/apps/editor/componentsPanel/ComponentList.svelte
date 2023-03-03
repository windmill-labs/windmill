<script lang="ts">
	import type { AppEditorContext } from '../../types'
	import { getContext } from 'svelte'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { components as componentsRecord, COMPONENT_SETS, type AppComponent } from '../component'
	import ListItem from './ListItem.svelte'
	import { insertNewGridItem } from '../appUtils'
	import { X } from 'lucide-svelte'

	const { app, selectedComponent, focusedGrid } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent(appComponentType: AppComponent['type']): void {
		$dirtyStore = true

		const data = componentsRecord[appComponentType].data
		const id = insertNewGridItem($app, data, $focusedGrid)

		$selectedComponent = id
	}

	let search = ''

	// Filter COMPONENT_SETS by search
	$: componentsFiltered = COMPONENT_SETS.map((set) => ({
		...set,
		components: set.components.filter((component) => {
			const name = componentsRecord[component].name.toLowerCase()
			return name.includes(search.toLowerCase())
		})
	}))
</script>

<section class="p-2 sticky bg-white border-b w-full h-12 z-20 top-0">
	<div class="relative">
		<input
			bind:value={search}
			class="px-2 py-1 border border-gray-300 rounded-sm {search ? 'pr-8' : ''}"
			placeholder="Search components..."
		/>
		{#if search}
			<button
				class="absolute right-2 top-1/2 transform -translate-y-1/2 hover:bg-gray-200 rounded-full p-0.5"
				on:click|stopPropagation|preventDefault={() => (search = '')}
			>
				<X size="14" />
			</button>
		{/if}
	</div>
</section>

{#if componentsFiltered.reduce((acc, { components }) => acc + components.length, 0) === 0}
	<div class="text-xs text-gray-500 py-1 px-2"> No components found </div>
{:else}
	{#each componentsFiltered as { title, components }, index (index)}
		<ListItem title={`${title} (${components.length})`}>
			{#if components.length}
				<div class="flex flex-wrap gap-2 py-2">
					{#each components as item}
						<button
							on:click={() => addComponent(item)}
							title={componentsRecord[item].name}
							class="border w-24 shadow-sm h-16 p-2 flex flex-col gap-2 items-center
							justify-center bg-white rounded-md hover:bg-gray-100 duration-200"
						>
							<svelte:component this={componentsRecord[item].icon} />
							<div class="text-xs w-full text-center ellipsize">
								{componentsRecord[item].name}
							</div>
						</button>
					{/each}
				</div>
			{:else}
				<div class="text-xs text-gray-500 py-1 px-2">
					There are no components in this group yet
				</div>
			{/if}
		</ListItem>
	{/each}
{/if}
