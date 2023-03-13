<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import { slide } from 'svelte/transition'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import { connectInput, sortGridItemsPosition } from '../appUtils'
	import ComponentOutput from './ComponentOutput.svelte'

	export let name: string | undefined = undefined
	export let parentId: string
	export let expanded: boolean = false
	export let subGrids: string[]

	const { app, connectingInput, breakpoint, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')

	let selected = 0

	$: outputs = $worldStore?.outputsById[parentId] as {
		selectedTabIndex: Output<number>
	}

	$: if (outputs?.selectedTabIndex) {
		outputs.selectedTabIndex.subscribe({
			next: (value) => {
				selected = value
			}
		})
	}
</script>

{#each subGrids as subGridId, index}
	<div class="ml-2 my-2">
		{#if subGrids.length > 1}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<div
				class={classNames(
					'px-1 py-0.5 flex justify-between items-center font-semibold text-xs border-l border-y w-full cursor-pointer',
					selected === index ? 'bg-gray-200' : 'bg-gray-50'
				)}
				on:click={() => {
					selected = index
				}}
			>
				<div class="text-xs">
					{name ? name : 'Should implement'}
					{index + 1}
				</div>
			</div>
		{/if}

		{#if selected === index || name !== 'Tabs'}
			<div transition:slide|local>
				{#if $app.subgrids && $app.subgrids[subGridId].length > 0}
					{#each sortGridItemsPosition($app.subgrids[subGridId], $breakpoint) as subGridItem, index}
						<ComponentOutput
							gridItem={subGridItem}
							first={index === 0}
							nested
							{expanded}
							on:select={({ detail }) => {
								$connectingInput = connectInput($connectingInput, subGridItem.id, detail)
							}}
						/>
					{/each}
				{:else}
					<div class="text-xs text-gray-500 border-y border-l p-1">No components</div>
				{/if}
			</div>
		{/if}
	</div>
{/each}
