<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let componentContainerHeight: number
	export let tabs: string[]
	export let customCss: ComponentCustomCSS<'tabscomponent'> | undefined = undefined
	export let render: boolean

	let resolvedConfig = initConfig(
		components['tabscomponent'].initialData.configuration,
		configuration
	)

	const { app, worldStore, focusedGrid, selectedComponent, mode, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let selected: string = tabs[0]
	let tabHeight: number = 0

	let outputs = initOutput($worldStore, id, {
		selectedTabIndex: 0
	})

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
</script>

<InputValue {id} input={configuration.tabsKind} bind:value={resolvedConfig.tabsKind} />

<div class={resolvedConfig.tabsKind == 'sidebar' ? 'flex gap-4 w-full' : 'w-full'}>
	{#if !resolvedConfig.tabsKind || resolvedConfig.tabsKind == 'tabs' || (resolvedConfig.tabsKind == 'invisibleOnView' && $mode == 'dnd')}
		<div bind:clientHeight={tabHeight}>
			<Tabs bind:selected class={css?.tabRow?.class} style={css?.tabRow?.style}>
				{#each tabs ?? [] as res}
					<Tab
						value={res}
						class={css?.allTabs?.class}
						style={css?.allTabs?.style}
						selectedClass={css?.selectedTab?.class}
						selectedStyle={css?.selectedTab?.style}
					>
						<span class="font-semibold">{res}</span>
					</Tab>
				{/each}
			</Tabs>
		</div>
	{:else if resolvedConfig.tabsKind == 'sidebar'}
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

	<div class="w-full">
		{#if $app.subgrids}
			{#each tabs ?? [] as res, i}
				<SubGridEditor
					{id}
					visible={render && i === selectedIndex}
					subGridId={`${id}-${i}`}
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
</div>
