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
	export let componentContainerHeight: number
	export let tabs: string[]

	export const staticOutputs: string[] = ['selectedTabIndex']
	const { app, worldStore, focusedGrid, selectedComponent } =
		getContext<AppEditorContext>('AppEditorContext')

	let selected: string = ''
	let noPadding: boolean | undefined = undefined

	$: selectedIndex = tabs?.indexOf(selected) ?? -1

	$: if ((tabs && selected === '') || !tabs.includes(selected)) {
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

	$: $selectedComponent === id && selectedIndex >= 0 && onFocus()
</script>

<InputValue {id} input={configuration.tabs} bind:value={tabs} />
<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />

<div>
	<div bind:clientHeight={tabHeight} on:pointerdown|stopPropagation>
		<Tabs bind:selected>
			{#each tabs ?? [] as res}
				<Tab value={res}>
					<span class="font-semibold">{res}</span>
				</Tab>
			{/each}
		</Tabs>
	</div>
	{#if $app.subgrids}
		{#each tabs ?? [] as res, i}
			<SubGridEditor
				visible={i === selectedIndex}
				bind:subGrid={$app.subgrids[`${id}-${i}`]}
				{noPadding}
				containerHeight={componentContainerHeight - tabHeight}
				on:focus={() => {
					$selectedComponent = id
				}}
			/>
		{/each}
	{/if}
</div>
