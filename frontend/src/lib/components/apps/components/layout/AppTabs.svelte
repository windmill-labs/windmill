<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let componentContainerHeight: number
	export let tabs: string[]
	export let customCss:
		| ComponentCustomCSS<'tabRow' | 'tabs' | 'selectedTab' | 'container'>
		| undefined = undefined
	export let render: boolean

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

	$: outputs?.selectedTabIndex && handleOutputs()

	function handleTabSelection() {
		outputs?.selectedTabIndex.set(selectedIndex)
		if ($selectedComponent != id) {
			$selectedComponent = id
		}
		if ($focusedGrid?.parentComponentId != id || $focusedGrid?.subGridIndex != selectedIndex) {
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: selectedIndex
			}
		}
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

	$: css = concatCustomCss($app.css?.tabscomponent, customCss)

	function handleOutputs() {
		outputs?.selectedTabIndex.set(outputs.selectedTabIndex.peak())
	}
</script>

<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />

<div>
	<div bind:clientHeight={tabHeight} on:pointerdown|stopPropagation>
		<Tabs bind:selected class={css?.tabRow?.class} style={css?.tabRow?.style}>
			{#each tabs ?? [] as res}
				<Tab
					value={res}
					class={css?.tabs?.class}
					style={css?.tabs?.style}
					selectedClass={css?.selectedTab?.class}
					selectedStyle={css?.selectedTab?.style}
				>
					<span class="font-semibold">{res}</span>
				</Tab>
			{/each}
		</Tabs>
	</div>
	{#if $app.subgrids}
		{#each tabs ?? [] as res, i}
			<SubGridEditor
				{id}
				visible={render && i === selectedIndex}
				bind:subGrid={$app.subgrids[`${id}-${i}`]}
				{noPadding}
				class={css?.container?.class}
				style={css?.container?.style}
				containerHeight={componentContainerHeight - tabHeight}
				on:focus={() => {
					$selectedComponent = id
				}}
			/>
		{/each}
	{/if}
</div>
