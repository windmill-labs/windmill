<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, GridItem } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string

	export let configuration: Record<string, AppInput>
	export let subGrids: GridItem[][] | undefined = undefined
	export let componentContainerHeight: number

	export const staticOutputs: string[] = ['selectedTabIndex']
	const { worldStore, focusedGrid, selectedComponent } =
		getContext<AppEditorContext>('AppEditorContext')

	let tabs: string[] | undefined = undefined
	let gridContent: string[] | undefined = undefined
	let selected: string = ''
	let noPadding: boolean | undefined = undefined

	$: selectedIndex = tabs?.indexOf(selected) ?? -1

	$: if (tabs && selected === '') {
		selected = tabs[0]
	}

	$: outputs = $worldStore?.outputsById[id] as {
		selectedTabIndex: Output<number | null>
	}

	function handleTabSelection() {
		outputs?.selectedTabIndex.set(selectedIndex)
	}

	$: selectedIndex >= 0 && handleTabSelection()
	let tabHeight: number = 0

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: selectedIndex ?? 0
		}
	}

	let previousTabs: string[] | undefined = tabs
	$: {
		if (tabs && previousTabs && tabs.length < previousTabs.length) {
			let missingTabIndex = tabs.findIndex((tab) => !previousTabs?.includes(tab))
		} else if ((tabs?.length ?? 0) > (previousTabs?.length ?? 0)) {
			// subGrids = [...(subGrids ?? []), []]
		}
		previousTabs = tabs
	}

	$: $selectedComponent === id && selectedIndex >= 0 && onFocus()
</script>

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />
<InputValue {id} input={configuration.tabs} bind:value={tabs} />
<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />

<div>
	<div bind:clientHeight={tabHeight}>
		<Tabs bind:selected>
			{#each tabs ?? [] as res}
				<Tab value={res}>
					<span class="font-semibold">{res}</span>
				</Tab>
			{/each}
		</Tabs>
	</div>
	{#if subGrids && subGrids[selectedIndex]}
		<SubGridEditor
			{noPadding}
			bind:subGrid={subGrids[selectedIndex]}
			containerHeight={componentContainerHeight - tabHeight}
			on:focus={() => {
				$selectedComponent = id
			}}
		/>
	{/if}
</div>
