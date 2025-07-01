<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import ComponentOutput from './ComponentOutput.svelte'

	interface Props {
		name?: string | undefined
		parentId: string
		expanded?: boolean
		subGrids: string[]
		nameOverrides?: string[] | undefined
		render: boolean
	}

	let {
		name = undefined,
		parentId,
		expanded = false,
		subGrids,
		nameOverrides = undefined,
		render
	}: Props = $props()
	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let selected = $state(0)

	function subscribeToOutput() {
		outputs.selectedTabIndex.subscribe(
			{
				id: 'subgridoutput-' + parentId,
				next: (value) => {
					selected = value
				}
			},
			selected
		)
	}
	let outputs = $derived(
		$worldStore?.outputsById[parentId] as {
			selectedTabIndex: Output<number>
		}
	)
	let subgridItems = $derived(
		subGrids.map((k) => ({
			k,
			items: $app.subgrids?.[k] ?? []
		}))
	)
	$effect(() => {
		if (outputs?.selectedTabIndex) {
			subscribeToOutput()
		}
	})
</script>

{#each subgridItems as { k, items }, index (k)}
	<div class="ml-2 my-2">
		{#if subGrids.length > 1 && render}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class={classNames(
					'px-1 py-0.5 flex justify-between items-center font-semibold text-xs border-l border-y w-full cursor-pointer',
					selected === index ? 'bg-surface-selected' : 'bg-surface'
				)}
				onclick={() => {
					selected = index
				}}
			>
				<div class="text-xs">
					{#if nameOverrides && nameOverrides[index]}
						{#key nameOverrides[index]}
							{nameOverrides[index]}
						{/key}
					{:else}
						{name ? name : 'Should implement name'}
						{index + 1}
					{/if}
				</div>
			</div>
		{/if}

		{#if selected === index || name !== 'Tabs'}
			<div class="border-l">
				{#if items.length > 0}
					{#each items as subGridItem, index (subGridItem.id)}
						<ComponentOutput
							renderRec={render}
							gridItem={subGridItem}
							first={index === 0}
							{expanded}
						/>
					{/each}
				{:else}
					<div class="text-xs text-tertiary border-y border-l p-1">No components</div>
				{/if}
			</div>
		{/if}
	</div>
{/each}
