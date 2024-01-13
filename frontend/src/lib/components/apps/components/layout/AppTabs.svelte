<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		RichConfiguration,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let componentContainerHeight: number
	export let tabs: string[]
	export let customCss: ComponentCustomCSS<'tabscomponent'> | undefined = undefined
	export let render: boolean
	export let disabledTabs: RichConfiguration[]

	let resolvedConfig = initConfig(
		components['tabscomponent'].initialData.configuration,
		configuration
	)

	const {
		app,
		worldStore,
		focusedGrid,
		selectedComponent,
		mode,
		componentControl,
		connectingInput
	} = getContext<AppViewerContext>('AppViewerContext')

	let selected: string = tabs[0]
	let tabHeight: number = 0

	let outputs = initOutput($worldStore, id, {
		selectedTabIndex: 0
	})

	function handleTabSelection() {
		selectedIndex = tabs?.indexOf(selected)
		outputs?.selectedTabIndex.set(selectedIndex)

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
			handleTabSelection()
		}
	}

	$: selected != undefined && handleTabSelection()
	let selectedIndex = tabs?.indexOf(selected) ?? -1
	let css = initCss($app.css?.tabscomponent, customCss)

	let resolvedDisabledTabs: boolean[] = []
</script>

<InputValue key="kind" {id} input={configuration.tabsKind} bind:value={resolvedConfig.tabsKind} />

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.tabscomponent}
	/>
{/each}

<InitializeComponent {id} />

{#each disabledTabs ?? [] as disableTab, index}
	<InputValue
		key="disable-{index}"
		{id}
		input={disableTab}
		bind:value={resolvedDisabledTabs[index]}
	/>
{/each}

<div class={resolvedConfig.tabsKind == 'sidebar' ? 'flex gap-4 w-full h-full' : 'w-full'}>
	{#if !resolvedConfig.tabsKind || resolvedConfig.tabsKind == 'tabs' || (resolvedConfig.tabsKind == 'invisibleOnView' && $mode == 'dnd')}
		<div bind:clientHeight={tabHeight}>
			<Tabs
				bind:selected
				class={twMerge(css?.tabRow?.class, 'wm-tabs-tabRow')}
				style={css?.tabRow?.style}
			>
				{#each tabs ?? [] as res, index}
					<Tab
						value={res}
						class={twMerge(css?.allTabs?.class, 'wm-tabs-alltabs')}
						style={css?.allTabs?.style}
						selectedClass={twMerge(css?.selectedTab?.class, 'wm-tabs-selectedTab')}
						selectedStyle={css?.selectedTab?.style}
						disabled={resolvedDisabledTabs[index]}
					>
						<span class="font-semibold">{res}</span>
					</Tab>
				{/each}
			</Tabs>
		</div>
	{:else if resolvedConfig.tabsKind == 'sidebar'}
		<div
			class={twMerge(
				'flex gap-y-2 flex-col w-1/6 max-w-[160px] bg-surface text-secondary opacity-80 px-4 pt-4 border-r ',
				css?.tabRow?.class,
				'wm-tabs-tabRow'
			)}
			style={css?.tabRow?.style}
		>
			{#each tabs ?? [] as res}
				<button
					on:pointerdown|stopPropagation
					on:click={() => (selected = res)}
					class={twMerge(
						'rounded-sm !truncate text-sm  hover:text-primary px-1 py-2',
						css?.allTabs?.class,
						'wm-tabs-alltabs',
						selected == res
							? twMerge(
									'border-r  border-primary border-l bg-surface text-primary',
									css?.selectedTab?.class,
									'wm-tabs-selectedTab'
							  )
							: ''
					)}
					style={selected == res
						? [css?.allTabs?.style, css?.selectedTab?.style].filter(Boolean).join(';')
						: css?.allTabs?.style}
				>
					{res}
				</button>
			{/each}
		</div>
	{/if}

	<div class="w-full">
		{#if $app.subgrids}
			{#each tabs ?? [] as _res, i}
				<SubGridEditor
					{id}
					visible={render && i === selectedIndex}
					subGridId={`${id}-${i}`}
					class={twMerge(css?.container?.class, 'wm-tabs-container')}
					style={css?.container?.style}
					containerHeight={resolvedConfig.tabsKind !== 'sidebar' && $mode !== 'preview'
						? componentContainerHeight - tabHeight
						: componentContainerHeight}
					on:focus={() => {
						if (!$connectingInput.opened) {
							$selectedComponent = [id]
							handleTabSelection()
						}
					}}
				/>
			{/each}
		{/if}
	</div>
</div>
