<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext } from 'svelte'
	import SubGridEditor from '../editor/SubGridEditor.svelte'
	import type { AppInput } from '../inputType'
	import type { Output } from '../rx'
	import type { AppEditorContext, GridItem } from '../types'
	import InputValue from './helpers/InputValue.svelte'
	import RunnableWrapper from './helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let configuration: Record<string, AppInput>
	export let subGrids: GridItem[][] | undefined = undefined
	export let componentContainerHeight: number

	export const staticOutputs: string[] = ['loading', 'result', 'selectedTabIndex']
	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let result: string[] | undefined = undefined
	let gridContent: string[] | undefined = undefined
	let selected: string = ''

	$: selectedIndex = result?.indexOf(selected) ?? -1

	$: if (result && selected === '') {
		selected = result[0]
	}

	$: outputs = $worldStore?.outputsById[id] as {
		selectedTabIndex: Output<number | null>
	}

	function handleTabSelection() {
		outputs?.selectedTabIndex.set(selectedIndex)
	}

	$: selectedIndex >= 0 && handleTabSelection()
	let tabHeight: number = 0
</script>

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />

<RunnableWrapper flexWrap bind:componentInput {id} bind:initializing bind:result>
	<div bind:clientHeight={tabHeight}>
		<Tabs bind:selected>
			{#each result ?? [] as res}
				<Tab value={res}>
					<span class="font-semibold">{res}</span>
				</Tab>
			{/each}
		</Tabs>
	</div>
	{#if subGrids && subGrids[selectedIndex]}
		<SubGridEditor
			bind:subGrid={subGrids[selectedIndex]}
			containerHeight={componentContainerHeight - tabHeight}
		/>
	{/if}
</RunnableWrapper>
