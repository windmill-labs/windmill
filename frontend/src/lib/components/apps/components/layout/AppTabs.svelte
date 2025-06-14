<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Tab, Tabs } from '$lib/components/common'
	import { getContext, untrack } from 'svelte'
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

	interface Props {
		id: string
		configuration: RichConfigurations
		componentContainerHeight: number
		tabs: string[]
		customCss?: ComponentCustomCSS<'tabscomponent'> | undefined
		render: boolean
		disabledTabs: RichConfiguration[]
		onTabChange?: string[] | undefined
	}

	let {
		id,
		configuration,
		componentContainerHeight,
		tabs,
		customCss = undefined,
		render,
		disabledTabs,
		onTabChange = undefined
	}: Props = $props()

	let resolvedConfig = $state(
		initConfig(components['tabscomponent'].initialData.configuration, configuration)
	)
	let everRender = $state(render)
	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	const {
		app,
		worldStore,
		focusedGrid,
		selectedComponent,
		mode,
		componentControl,
		connectingInput,
		runnableComponents
	} = getContext<AppViewerContext>('AppViewerContext')

	let selected: string = $state(tabs[0])
	let selectedIndex = $state(tabs?.indexOf(selected) ?? -1)
	let css = $state(initCss($app.css?.tabscomponent, customCss))

	let tabHeight: number = $state(0)

	let outputs = initOutput($worldStore, id, {
		selectedTabIndex: 0
	})

	const titleBarHeight = 40 // Example height of a single accordion title bar in pixels

	function handleTabSelection() {
		let oldSelectedIndex = selectedIndex
		selectedIndex = tabs?.indexOf(selected)
		if (selectedIndex == -1) {
			selectedIndex = oldSelectedIndex
			if (selectedIndex != -1 && tabs.length > 0) {
				selected = tabs[selectedIndex]
			}
		}
		outputs?.selectedTabIndex.set(selectedIndex)

		onTabChange?.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb?.()))

		if ($focusedGrid?.parentComponentId != id || $focusedGrid?.subGridIndex != selectedIndex) {
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: selectedIndex
			}
		}
	}

	$componentControl[id] = {
		setSelectedIndex(index) {
			if (index >= 0 && index < tabs.length) {
				selected = tabs[index]
			}
		},
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

	$effect.pre(() => {
		tabs
		selected != undefined && untrack(() => handleTabSelection())
	})

	let resolvedDisabledTabs: boolean[] = $state([])
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
		key="tabDisabled {index}"
		{id}
		input={disableTab}
		bind:value={resolvedDisabledTabs[index]}
	/>
{/each}

{#if everRender}
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
				{#each tabs ?? [] as res, index}
					<button
						onpointerdown={stopPropagation(bubble('pointerdown'))}
						onclick={() => !resolvedDisabledTabs[index] && (selected = res)}
						disabled={resolvedDisabledTabs[index]}
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
								: '',
							resolvedDisabledTabs[index]
								? 'opacity-50 cursor-not-allowed hover:text-secondary'
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
		{#if resolvedConfig.tabsKind == 'accordion'}
			<div class="flex flex-col w-full">
				{#each tabs ?? [] as res, index}
					<div class="border-b">
						<button
							onpointerdown={stopPropagation(bubble('pointerdown'))}
							onclick={() => !resolvedDisabledTabs[index] && (selected = res)}
							disabled={resolvedDisabledTabs[index]}
							class={twMerge(
								'w-full text-left bg-surface !truncate text-sm hover:text-primary px-1 py-2',
								css?.allTabs?.class,
								'wm-tabs-alltabs',
								selected == res
									? twMerge(
											'bg-surface text-primary ',
											css?.selectedTab?.class,
											'wm-tabs-selectedTab'
										)
									: 'text-secondary',
								resolvedDisabledTabs[index]
									? 'opacity-50 cursor-not-allowed hover:text-secondary'
									: ''
							)}
						>
							<span class="mr-2 w-8 font-mono">{selected == res ? '-' : '+'}</span>
							{res}
						</button>
						<div class={selected == res ? 'border-t' : ''}>
							<SubGridEditor
								{id}
								visible={render && index === selectedIndex}
								subGridId={`${id}-${index}`}
								class={twMerge(css?.container?.class, 'wm-tabs-container')}
								style={css?.container?.style}
								containerHeight={componentContainerHeight - (titleBarHeight * tabs.length + 40)}
								on:focus={() => {
									if (!$connectingInput.opened) {
										$selectedComponent = [id]
										handleTabSelection()
									}
								}}
							/>
						</div>
					</div>
				{/each}
			</div>
		{:else}
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
		{/if}
	</div>
{:else if $app.subgrids}
	{#each tabs ?? [] as _res, i}
		<SubGridEditor {id} visible={false} subGridId={`${id}-${i}`} />
	{/each}
{/if}
