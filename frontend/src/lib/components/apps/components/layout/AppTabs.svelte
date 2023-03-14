<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, AppViewerContext, ComponentCustomCSS } from '../../types'
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
	export let initializing: boolean | undefined = configuration.tabsKind != undefined

	export const staticOutputs: string[] = ['selectedTabIndex']
	const { app, worldStore, focusedGrid, selectedComponent, mode, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let selected: string = tabs[0]
	let kind: 'tabs' | 'sidebar' | 'invisibleOnView' | undefined = undefined
	let tabHeight: number = 0

	$: outputs = $worldStore
		? initOutput($worldStore, id, {
				selectedTabIndex: 0
		  })
		: undefined

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
		},
		setTab: (tab: number) => {
			selected = tabs[tab]
		}
	}

	$: selected && handleTabSelection()

	$: selectedIndex = tabs?.indexOf(selected) ?? -1

	$: css = concatCustomCss($app.css?.tabscomponent, customCss)

	let subgridWidth: number | undefined = undefined
</script>

<InputValue
	on:done={() => initializing && (initializing = false)}
	{id}
	input={configuration.tabsKind}
	bind:value={kind}
/>

<div class={kind == 'sidebar' ? 'flex gap-4 w-full' : 'w-full'}>
	{#if !kind || kind == 'tabs' || (kind == 'invisibleOnView' && $mode == 'dnd')}
		<div bind:clientHeight={tabHeight}>
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
	{:else if kind == 'sidebar'}
		<div
			class="flex gap-y-2 flex-col w-1/6 max-w-[160px] bg-white text-[#2e3440] opacity-80 px-4 pt-4 border-r border-gray-400"
		>
			{#each tabs ?? [] as res}
				<button
					class="rounded-sm !truncate text-sm hover:bg-gray-100 hover:border hover:text-black px-1 py-2 {selected ==
					res
						? 'outline outline-gray-500 outline-1 bg-white text-black'
						: ''}"
					on:pointerdown|stopPropagation
					on:click={() => (selected = res)}>{res}</button
				>
			{/each}
		</div>
	{/if}

	<div bind:clientWidth={subgridWidth} class="w-full">
		{#if $app.subgrids}
			{#each tabs ?? [] as res, i}<SubGridEditor
					{id}
					visible={render && i === selectedIndex}
					bind:subGrid={$app.subgrids[`${id}-${i}`]}
					class={css?.container?.class}
					style={css?.container?.style}
					containerHeight={componentContainerHeight - tabHeight}
					on:focus={() => {
						$selectedComponent = id
					}}
				/>{/each}
		{/if}
	</div>
</div>
