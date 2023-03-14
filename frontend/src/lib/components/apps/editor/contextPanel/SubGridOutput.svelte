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

	let sortedItems = subGrids.map((k) => ({
		k,
		items: sortGridItemsPosition($app.subgrids?.[k] ?? [], $breakpoint)
	}))

	$: $app.subgrids &&
		setTimeout(
			() =>
				(sortedItems = subGrids.map((k) => ({
					k,
					items: sortGridItemsPosition($app.subgrids?.[k] ?? [], $breakpoint)
				}))),
			500
		)

	$: if (outputs?.selectedTabIndex) {
		outputs.selectedTabIndex.subscribe({
			id: 'subgridoutput-' + parentId,
			next: (value) => {
				selected = value
			}
		})
	}
</script>

{#each sortedItems as { k, items }, index (k)}
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
					{name ? name : 'Should implement name'}
					{index + 1}
				</div>
			</div>
		{/if}

		{#if selected === index || name !== 'Tabs'}
			<div transition:slide|local class="border-l">
				{#if items.length > 0}
					{#each items as subGridItem, index (subGridItem.id)}
						<ComponentOutput
							gridItem={subGridItem}
							first={index === 0}
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
