<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext, onMount } from 'svelte'
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
	const { app, worldStore, focusedGrid, selectedComponent, componentControl } =
		getContext<AppEditorContext>('AppEditorContext')

	let selected: string = tabs[0]
	let noPadding: boolean | undefined = undefined
	let tabHeight: number = 0

	function handleTabSelection() {
		const selectedIndex = tabs?.indexOf(selected)
		outputs?.selectedTabIndex.set(selectedIndex, true)

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

	$: $selectedComponent === id && focusGrid()

	function focusGrid() {
		const selectedIndex = tabs?.indexOf(selected)
		if ($focusedGrid?.parentComponentId != id || $focusedGrid?.subGridIndex != selectedIndex) {
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: selectedIndex
			}
		}
	}

	$componentControl[id] = {
		left: () => {
			const index = tabs.indexOf(selected)
			if (index > 0) {
				selected = tabs[index - 1]
				return true
			}
			return false
		},
		right: () => {
			const index = tabs.indexOf(selected)
			if (index < tabs.length - 1) {
				selected = tabs[index + 1]
				return true
			}
			return false
		}
	}

	$: selected && handleTabSelection()

	$: outputs = $worldStore?.outputsById[id] as {
		selectedTabIndex: Output<number | null>
	}

	$: selectedIndex = tabs?.indexOf(selected) ?? -1

	$: css = concatCustomCss($app.css?.tabscomponent, customCss)
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
