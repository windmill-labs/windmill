<script lang="ts">
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import { getContext } from 'svelte'
	import { fade, slide } from 'svelte/transition'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import {
		components as componentsRecord,
		COMPONENT_SETS,
		type AppComponent,
		type TypedComponent
	} from '../component'
	import ListItem from './ListItem.svelte'
	import { appComponentFromType, insertNewGridItem } from '../appUtils'
	import { X } from 'lucide-svelte'
	import { push } from '$lib/history'
	import { flip } from 'svelte/animate'

	const { app, selectedComponent, focusedGrid, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')

	const { history } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent(appComponentType: TypedComponent['type']): void {
		push(history, $app)

		$dirtyStore = true

		const id = insertNewGridItem(
			$app,
			appComponentFromType(appComponentType) as (id: string) => AppComponent,
			$focusedGrid
		)

		$selectedComponent = [id]
		$app = $app
		$worldStore = $worldStore
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

<section class="p-2 sticky bg-white w-full z-10 top-0">
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

<div class="relative">
	{#if componentsFiltered.reduce((acc, { components }) => acc + components.length, 0) === 0}
		<div
			in:fade|local={{ duration: 50, delay: 50 }}
			out:fade|local={{ duration: 50 }}
			class="absolute left-0 top-0 w-full text-sm text-gray-500 text-center py-6 px-2"
		>
			No components found
		</div>
	{:else}
		<div in:fade|local={{ duration: 50, delay: 50 }} out:fade|local={{ duration: 50 }}>
			{#each componentsFiltered as { title, components }, index (index)}
				{#if components.length}
					<div transition:slide|local={{ duration: 100 }}>
						<ListItem title={`${title} (${components.length})`}>
							<div class="flex flex-wrap gap-3 py-2">
								{#each components as item (item)}
									<div animate:flip={{ duration: 100 }} class="w-20">
										<button
											on:click={() => addComponent(item)}
											title={componentsRecord[item].name}
											class="transition-all border w-20 shadow-sm h-16 p-2 flex flex-col gap-2 items-center
											justify-center bg-white rounded-md hover:bg-blue-50 duration-200 hover:border-blue-500"
										>
											<svelte:component this={componentsRecord[item].icon} class="text-gray-600" />
										</button>
										<div class="text-xs text-center flex-wrap text-gray-600 mt-1">
											{componentsRecord[item].name}
										</div>
									</div>
								{/each}
							</div>
						</ListItem>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
