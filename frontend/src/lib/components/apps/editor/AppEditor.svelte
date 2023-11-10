<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { afterUpdate, onMount, setContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import { buildWorld } from '../rx'
	import type {
		App,
		AppEditorContext,
		AppViewerContext,
		ConnectingInput,
		ContextPanelContext,
		EditorBreakpoint,
		EditorMode,
		FocusedGrid
	} from '../types'
	import AppEditorHeader from './AppEditorHeader.svelte'
	import GridEditor from './GridEditor.svelte'

	import { Button, Tab } from '$lib/components/common'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { classNames, encodeState } from '$lib/utils'
	import AppPreview from './AppPreview.svelte'
	import ComponentList from './componentsPanel/ComponentList.svelte'
	import ContextPanel from './contextPanel/ContextPanel.svelte'

	import InlineScriptsPanel from './inlineScriptsPanel/InlineScriptsPanel.svelte'

	import { page } from '$app/stores'
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import { VariableService, type Policy } from '$lib/gen'
	import { initHistory } from '$lib/history'
	import { Component, Minus, Paintbrush, Plus } from 'lucide-svelte'
	import { findGridItem, findGridItemParentGrid } from './appUtils'
	import ComponentNavigation from './component/ComponentNavigation.svelte'
	import CssSettings from './componentsPanel/CssSettings.svelte'
	import ConnectionInstructions from './ConnectionInstructions.svelte'
	import SettingsPanel from './SettingsPanel.svelte'
	import {
		SecondaryMenu,
		secondaryMenuLeft,
		secondaryMenuLeftStore,
		secondaryMenuRight,
		secondaryMenuRightStore
	} from './settingsPanel/secondaryMenu'
	import Popover from '../../Popover.svelte'
	import { BG_PREFIX, migrateApp } from '../utils'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { getTheme } from './componentsPanel/themeUtils'
	import StylePanel from './settingsPanel/StylePanel.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'

	export let app: App
	export let path: string
	export let policy: Policy
	export let summary: string
	export let fromHub: boolean = false
	export let versions: number[]
	export let diffDrawer: DiffDrawer | undefined = undefined
	export let savedApp:
		| {
				value: App
				draft?: any
				path: string
				summary: string
				policy: any
				draft_only?: boolean
		  }
		| undefined = undefined

	migrateApp(app)

	const appStore = writable<App>(app)
	const selectedComponent = writable<string[] | undefined>(undefined)
	const mode = writable<EditorMode>('dnd')
	const breakpoint = writable<EditorBreakpoint>('lg')
	const summaryStore = writable(summary)
	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined,
		hoveredComponent: undefined
	})

	const cssEditorOpen = writable<boolean>(false)

	const history = initHistory(app)

	const jobsById = writable<
		Record<
			string,
			{
				job: string
				component: string
				result?: string
				transformer?: { result?: string; error?: string }
				started_at?: number
				duration_ms?: number
			}
		>
	>({})
	const focusedGrid = writable<FocusedGrid | undefined>(undefined)
	const pickVariableCallback: Writable<((path: string) => void) | undefined> = writable(undefined)
	let context = {
		email: $userStore?.email,
		groups: $userStore?.groups,
		username: $userStore?.username,
		query: Object.fromEntries($page.url.searchParams.entries()),
		hash: $page.url.hash,
		workspace: $workspaceStore,
		mode: 'editor'
	}
	const darkMode: Writable<boolean> = writable(document.documentElement.classList.contains('dark'))

	const worldStore = buildWorld(context)
	const previewTheme: Writable<string | undefined> = writable(undefined)

	$secondaryMenuRightStore.isOpen = false
	$secondaryMenuLeftStore.isOpen = false

	setContext<AppViewerContext>('AppViewerContext', {
		worldStore,
		app: appStore,
		summary: summaryStore,
		initialized: writable({ initialized: false, initializedComponents: [] }),
		selectedComponent,
		mode,
		connectingInput,
		breakpoint,
		runnableComponents: writable({}),
		appPath: path,
		workspace: $workspaceStore ?? '',
		onchange: () => saveFrontendDraft(),
		isEditor: true,
		jobs: writable([]),
		staticExporter: writable({}),
		noBackend: false,
		errorByComponent: writable({}),
		jobsById,
		openDebugRun: writable(undefined),
		focusedGrid,
		stateId: writable(0),
		parentWidth: writable(0),
		state: writable({}),
		componentControl: writable({}),
		hoverStore: writable(undefined),
		allIdsInPath: writable([]),
		darkMode,
		cssEditorOpen,
		previewTheme
	})

	let scale = writable(100)

	const componentActive = writable(false)

	let yTop = writable(0)

	setContext<AppEditorContext>('AppEditorContext', {
		yTop,
		componentActive,
		dndItem: writable({}),
		refreshComponents: writable(undefined),
		history,
		pickVariableCallback,
		movingcomponents: writable(undefined),
		selectedComponentInEditor: writable(undefined),
		jobsDrawerOpen: writable(false),
		scale
	})

	let timeout: NodeJS.Timeout | undefined = undefined

	$: $appStore && saveFrontendDraft()

	function saveFrontendDraft() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			try {
				localStorage.setItem(path != '' ? `app-${path}` : 'app', encodeState($appStore))
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	function hashchange(e: HashChangeEvent) {
		context.hash = e.newURL.split('#')[1]
		context = context
	}

	$: context.mode = $mode == 'dnd' ? 'editor' : 'viewer'

	$: width = $breakpoint === 'sm' ? 'min-w-[400px] max-w-[656px]' : 'min-w-[710px] w-full'

	let selectedTab: 'insert' | 'settings' | 'css' = 'insert'

	let befSelected: string | undefined = undefined

	$: if ($selectedComponent?.[0] != befSelected && $selectedComponent?.[0] != undefined) {
		befSelected = $selectedComponent?.[0]
		selectedTab = 'settings'

		if (befSelected) {
			if (!['ctx', 'state'].includes(befSelected) && !befSelected?.startsWith(BG_PREFIX)) {
				let item = findGridItem($appStore, befSelected)
				if (item?.data.type === 'containercomponent' || item?.data.type === 'listcomponent') {
					$focusedGrid = {
						parentComponentId: befSelected,
						subGridIndex: 0
					}
				} else if (item?.data.type === 'tabscomponent' || item?.data.type === 'steppercomponent') {
					$focusedGrid = {
						parentComponentId: befSelected,
						subGridIndex:
							($worldStore.outputsById?.[befSelected]?.selectedTabIndex?.peak() as number) ?? 0
					}
				} else {
					let subgrid = findGridItemParentGrid($appStore, befSelected)
					if (subgrid) {
						try {
							$focusedGrid = {
								parentComponentId: subgrid.split('-')[0],
								subGridIndex: parseInt(subgrid.split('-')[1])
							}
						} catch {}
					} else {
						$focusedGrid = undefined
					}
				}
			}
		}
	}

	let itemPicker: ItemPicker | undefined = undefined

	$: if ($pickVariableCallback) {
		itemPicker?.openDrawer()
	}

	let variableEditor: VariableEditor | undefined = undefined

	setContext<ContextPanelContext>('ContextPanel', {
		search: writable<string>(''),
		manuallyOpened: writable<Record<string, boolean>>({}),
		hasResult: writable<Record<string, boolean>>({})
	})

	$: if ($connectingInput.opened) {
		secondaryMenuRight.open(ConnectionInstructions, {}, () => {
			$connectingInput.opened = false
		})
		secondaryMenuLeft.close()
	} else {
		secondaryMenuRight.close()
	}

	function onThemeChange() {
		$darkMode = document.documentElement.classList.contains('dark')
	}

	let runnablePanelSize = 30
	let gridPanelSize = 70

	let leftPanelSize = 22
	let centerPanelSize = 63
	let rightPanelSize = 22

	let tmpRunnablePanelSize = -1
	let tmpGridPanelSize = -1

	let tmpLeftPanelSize = -1
	let tmpCenterPanelSize = -1
	let tmpRightPanelSize = -1

	let toggled = false
	let cssToggled = false

	$: if ($connectingInput.opened && !toggled) {
		tmpRunnablePanelSize = runnablePanelSize
		tmpGridPanelSize = gridPanelSize

		animateTo(runnablePanelSize, 0, (newValue: number) => (runnablePanelSize = newValue))
		animateTo(gridPanelSize, 100, (newValue: number) => (gridPanelSize = newValue))

		toggled = true
	} else if (!$connectingInput.opened && toggled) {
		animateTo(
			runnablePanelSize,
			tmpRunnablePanelSize,
			(newValue: number) => (runnablePanelSize = newValue)
		)
		animateTo(gridPanelSize, tmpGridPanelSize, (newValue: number) => (gridPanelSize = newValue))

		tmpRunnablePanelSize = -1
		tmpGridPanelSize = -1

		toggled = false
	}

	// Animation logic for cssInput
	$: animateCssInput($cssEditorOpen)
	$: $cssEditorOpen && secondaryMenuLeft?.open(StylePanel, {})

	function animateCssInput(cssEditorOpen: boolean) {
		if (cssEditorOpen && !cssToggled) {
			cssToggled = true

			tmpLeftPanelSize = leftPanelSize
			tmpCenterPanelSize = centerPanelSize
			tmpRightPanelSize = rightPanelSize

			animateTo(leftPanelSize, 20, (newValue: number) => (leftPanelSize = newValue))
			animateTo(centerPanelSize, 55, (newValue: number) => (centerPanelSize = newValue))
			animateTo(rightPanelSize, 25, (newValue: number) => (rightPanelSize = newValue))

			tmpRunnablePanelSize = runnablePanelSize
			tmpGridPanelSize = gridPanelSize

			animateTo(runnablePanelSize, 0, (newValue: number) => (runnablePanelSize = newValue))
			animateTo(gridPanelSize, 100, (newValue: number) => (gridPanelSize = newValue))
		} else if (!cssEditorOpen && cssToggled) {
			cssToggled = false

			animateTo(leftPanelSize, tmpLeftPanelSize, (newValue: number) => (leftPanelSize = newValue))
			animateTo(
				centerPanelSize,
				tmpCenterPanelSize,
				(newValue: number) => (centerPanelSize = newValue)
			)
			animateTo(
				rightPanelSize,
				tmpRightPanelSize,
				(newValue: number) => (rightPanelSize = newValue)
			)

			tmpLeftPanelSize = -1
			tmpCenterPanelSize = -1
			tmpRightPanelSize = -1

			animateTo(
				runnablePanelSize,
				tmpRunnablePanelSize,
				(newValue: number) => (runnablePanelSize = newValue)
			)
			animateTo(gridPanelSize, tmpGridPanelSize, (newValue: number) => (gridPanelSize = newValue))

			tmpRunnablePanelSize = -1
			tmpGridPanelSize = -1
		}
	}

	function animateTo(start: number, end: number, onUpdate: (newValue: number) => void) {
		const duration = 400
		const startTime = performance.now()

		function animate(time: number) {
			const elapsed = time - startTime
			const progress = Math.min(elapsed / duration, 1)
			const currentValue = start + (end - start) * easeInOut(progress)
			onUpdate(currentValue)
			if (progress < 1) {
				requestAnimationFrame(animate)
			}
		}

		requestAnimationFrame(animate)
	}

	function easeInOut(t: number) {
		return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t
	}

	$: $cssEditorOpen && selectCss()

	function selectCss() {
		selectedTab !== 'css' && (selectedTab = 'css')
	}

	const cssId = 'wm-global-style'

	$: addOrRemoveCss(true, $mode === 'preview')

	let css: string | undefined = undefined

	let lastTheme: string | undefined = undefined
	appStore.subscribe(async (currentAppStore) => {
		if (!currentAppStore.theme) {
			return
		}

		if (JSON.stringify(currentAppStore.theme) != lastTheme) {
			if (currentAppStore.theme.type === 'inlined') {
				css = currentAppStore.theme.css
			} else if (currentAppStore.theme.type === 'path' && currentAppStore.theme?.path) {
				let loadedCss = await getTheme($workspaceStore!, currentAppStore.theme.path)
				css = loadedCss.value
			}
			lastTheme = JSON.stringify(currentAppStore.theme)
		}
	})

	$: updateCssContent(css, $previewTheme)

	function addOrRemoveCss(isPremium: boolean, isPreview: boolean = false) {
		const existingElement = document.getElementById(cssId)

		if (!isPremium && isPreview) {
			if (existingElement) {
				existingElement.remove()
			}
		} else {
			if (!existingElement) {
				const head = document.head
				const link = document.createElement('style')
				link.id = cssId
				link.innerHTML = css ?? ''
				head.appendChild(link)
			}
		}
	}

	function updateCssContent(cssString: string | undefined, previewTheme: string | undefined) {
		const theme = previewTheme ?? cssString ?? ''

		const existingElement = document.getElementById(cssId)
		if (existingElement && theme !== existingElement.innerHTML) {
			existingElement.innerHTML = theme
		}
	}

	let appEditorHeader: AppEditorHeader | undefined = undefined

	export function triggerTutorial() {
		appEditorHeader?.toggleTutorial()
	}

	let box
	function parseScroll() {
		$yTop = box?.scrollTop
	}

	let mounted = false
	onMount(() => {
		mounted = true

		parseScroll()
	})

	$: setGridPanelSize($componentActive)

	let lastComponentActive = false

	afterUpdate(() => {
		if ($componentActive != lastComponentActive) {
			box.scrollTop = $yTop
			lastComponentActive = $componentActive
		}
	})

	function setGridPanelSize(x: boolean) {
		if (mounted) {
			if (x) {
				gridPanelSize = 100
			} else {
				gridPanelSize = 100 - runnablePanelSize
			}
		}
	}
</script>

<DarkModeObserver on:change={onThemeChange} />

<svelte:window on:hashchange={hashchange} />

{#if !$userStore?.operator}
	{#if $appStore}
		<AppEditorHeader
			on:restore
			{versions}
			{policy}
			{fromHub}
			bind:this={appEditorHeader}
			{diffDrawer}
			bind:savedApp
		/>
		{#if $mode === 'preview'}
			<SplitPanesWrapper>
				<div
					class={twMerge(
						'h-full w-full relative',
						$appStore.css?.['app']?.['viewer']?.class,
						'wm-app-viewer'
					)}
					style={$appStore.css?.['app']?.['viewer']?.style}
				>
					<AppPreview
						workspace={$workspaceStore ?? ''}
						summary={$summaryStore}
						app={$appStore}
						appPath={path}
						{breakpoint}
						{policy}
						isEditor
						{context}
						noBackend={false}
					/>
				</div>
			</SplitPanesWrapper>
		{:else}
			{#if $componentActive}
				<div
					class="absolute z-50 inset-0 h-full w-full bg-surface-secondary [background-size:16px_16px]"
				/>
			{/if}

			<SplitPanesWrapper>
				<Splitpanes id="o1" class="max-w-full">
					<Pane bind:size={leftPanelSize} minSize={5} maxSize={33}>
						<div
							class={twMerge(
								'w-full h-full relative',
								$secondaryMenuLeftStore.isOpen ? 'overflow-hidden' : ''
							)}
						>
							<!-- {yTop} -->

							<SecondaryMenu right={false} />
							<ContextPanel />
						</div>
					</Pane>
					<Pane bind:size={centerPanelSize}>
						<Splitpanes id="o2" horizontal class="!overflow-visible">
							<Pane bind:size={gridPanelSize} class="ovisible">
								<div
									on:pointerdown={(e) => {
										$selectedComponent = undefined
										$focusedGrid = undefined
									}}
									class={twMerge(
										'bg-surface-secondary h-full w-full relative',
										$appStore.css?.['app']?.['viewer']?.class,
										'wm-app-viewer z-[100]  h-full overflow-visible'
									)}
									style={$appStore.css?.['app']?.['viewer']?.style}
								>
									<div class="absolute bottom-2 left-4 z-50">
										<div class="flex flex-row gap-2 text-xs items-center">
											<Button
												color="light"
												variant="border"
												size="xs2"
												disabled={$scale <= 30}
												on:click={() => {
													$scale -= 10
												}}
											>
												<Minus size={14} />
											</Button>
											{$scale}%
											<Button
												color="light"
												variant="border"
												size="xs2"
												disabled={$scale >= 100}
												on:click={() => {
													$scale += 10
												}}
											>
												<Plus size={14} />
											</Button>
										</div>
									</div>

									<div id="app-editor-top-level-drawer" />
									<div
										class="absolute inset-0 h-full w-full surface-secondary bg-[radial-gradient(#dbdbdb_1px,transparent_1px)] dark:bg-[radial-gradient(#666666_1px,transparent_1px)] [background-size:16px_16px]"
									/>

									<!-- svelte-ignore a11y-no-static-element-interactions -->
									<div
										bind:this={box}
										on:scroll={parseScroll}
										class={classNames(
											'mx-auto w-full h-full z-50',
											$appStore.fullscreen ? '' : 'max-w-7xl border-x',
											$componentActive ? 'absolute' : 'overflow-auto'
										)}
										style={$componentActive ? `top: -${$yTop}px;` : ''}
									>
										{#if $appStore.grid}
											<ComponentNavigation />

											<div
												on:pointerdown|stopPropagation
												class={twMerge(width, 'mx-auto', 'z-10000')}
											>
												<GridEditor {policy} />
											</div>
										{/if}
									</div>
								</div>
							</Pane>
							{#if $connectingInput?.opened == false && !$componentActive}
								<Pane bind:size={runnablePanelSize}>
									<div class="relative h-full w-full">
										<InlineScriptsPanel />
									</div>
								</Pane>
							{/if}
						</Splitpanes>
					</Pane>
					<Pane bind:size={rightPanelSize} minSize={15} maxSize={33}>
						<div class="relative flex flex-col h-full">
							<Tabs bind:selected={selectedTab} wrapperClass="!min-h-[42px]" class="!h-full">
								<Popover disappearTimeout={0} notClickable placement="bottom">
									<svelte:fragment slot="text">Component library</svelte:fragment>
									<Tab
										value="insert"
										size="xs"
										class="h-full"
										on:pointerdown={() => {
											if ($cssEditorOpen) {
												$cssEditorOpen = false
												selectedTab = 'insert'
											}
										}}
										id="app-editor-component-library-tab"
									>
										<div class="m-1 center-center">
											<Plus size={18} />
										</div>
									</Tab>
								</Popover>
								<Popover disappearTimeout={0} notClickable placement="bottom">
									<svelte:fragment slot="text">Component settings</svelte:fragment>
									<Tab
										value="settings"
										size="xs"
										class="h-full"
										on:pointerdown={() => {
											if ($cssEditorOpen) {
												$cssEditorOpen = false
												selectedTab = 'settings'
											}
										}}
									>
										<div class="m-1 center-center">
											<Component size={18} />
										</div>
									</Tab>
								</Popover>
								<Popover disappearTimeout={0} notClickable placement="bottom">
									<svelte:fragment slot="text">Global styling</svelte:fragment>
									<Tab
										value="css"
										size="xs"
										class="h-full"
										on:pointerdown={() => {
											if (!$cssEditorOpen) {
												$cssEditorOpen = true
												selectedTab = 'css'
											}
										}}
									>
										<div class="m-1 center-center">
											<Paintbrush size={18} />
										</div>
									</Tab>
								</Popover>
								<div slot="content" class="h-full overflow-y-auto">
									<TabContent class="overflow-auto h-full" value="settings">
										{#if $selectedComponent !== undefined}
											<SettingsPanel
												on:delete={() => {
													befSelected = undefined
													selectedTab = 'insert'
												}}
											/>
											<SecondaryMenu right />
										{:else}
											<div class="min-w-[150px] text-sm !text-secondary text-center py-8 px-2">
												Select a component to see the settings&nbsp;for&nbsp;it
											</div>
										{/if}
									</TabContent>
									<TabContent value="insert">
										<ComponentList />
									</TabContent>
									<TabContent value="css" class="h-full">
										<CssSettings />
									</TabContent>
								</div>
							</Tabs>
						</div>
					</Pane>
				</Splitpanes>
			</SplitPanesWrapper>
		{/if}
	{:else}
		App is null
	{/if}
{:else}
	App editor not available to operators
{/if}

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, _) => {
		$pickVariableCallback?.(path)
	}}
	itemName="Variable"
	extraField="path"
	loadItems={async () =>
		(await VariableService.listVariable({ workspace: $workspaceStore ?? '' })).map((x) => ({
			name: x.path,
			...x
		}))}
>
	<div
		slot="submission"
		class="flex flex-row-reverse w-full bg-surface border-t border-gray-200 rounded-bl-lg rounded-br-lg"
	>
		<Button
			variant="border"
			color="blue"
			size="sm"
			startIcon={{ icon: Plus }}
			on:click={() => {
				variableEditor?.initNew?.()
			}}
		>
			New Variable
		</Button>
	</div>
</ItemPicker>

<VariableEditor bind:this={variableEditor} />

<style global>
	#o1 > .splitpanes__pane {
		overflow: visible !important;
	}
	#o2 > .splitpanes__pane {
		overflow: visible !important;
	}
</style>
