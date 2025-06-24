<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { onMount, setContext, untrack } from 'svelte'
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

	import { Alert, Button, Tab } from '$lib/components/common'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/TabsV2.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		classNames,
		EDITOR_POSITION_MAP_CONTEXT_KEY,
		type EditorPositionMap,
		encodeState,
		getModifierKey,
		sendUserToast
	} from '$lib/utils'
	import AppPreview from './AppPreview.svelte'
	import ComponentList from './componentsPanel/ComponentList.svelte'
	import ContextPanel from './contextPanel/ContextPanel.svelte'

	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import { VariableService, type Policy } from '$lib/gen'
	import { initHistory } from '$lib/history'
	import { Component, Minus, Paintbrush, Plus, Smartphone, Scan, Hand, Grab } from 'lucide-svelte'
	import { animateTo, findGridItem, findGridItemParentGrid } from './appUtils'
	import ComponentNavigation from './component/ComponentNavigation.svelte'
	import CssSettings from './componentsPanel/CssSettings.svelte'
	import SettingsPanel from './SettingsPanel.svelte'
	import {
		SecondaryMenu,
		secondaryMenuLeft,
		secondaryMenuLeftStore,
		secondaryMenuRightStore
	} from './settingsPanel/secondaryMenu'
	import Popover from '../../Popover.svelte'
	import { BG_PREFIX, migrateApp } from '../utils'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { getTheme } from './componentsPanel/themeUtils'
	import StylePanel from './settingsPanel/StylePanel.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import HideButton from './settingsPanel/HideButton.svelte'
	import AppEditorBottomPanel from './AppEditorBottomPanel.svelte'
	import panzoom from 'panzoom'

	interface Props {
		app: App
		path: string
		policy: Policy
		summary: string
		fromHub?: boolean
		diffDrawer?: DiffDrawer | undefined
		savedApp?:
			| {
					value: App
					draft?: any
					path: string
					summary: string
					policy: any
					draft_only?: boolean
					custom_path?: string
			  }
			| undefined
		version?: number | undefined
		newApp?: boolean
		newPath?: string | undefined
		replaceStateFn?: (path: string) => void
		gotoFn?: (path: string, opt?: Record<string, any> | undefined) => void
		unsavedConfirmationModal?: import('svelte').Snippet<[any]>
	}

	let {
		app,
		path,
		policy,
		summary,
		fromHub = false,
		diffDrawer = undefined,
		savedApp = $bindable(undefined),
		version = undefined,
		newApp = false,
		newPath = undefined,
		replaceStateFn = (path: string) => window.history.replaceState(null, '', path),
		gotoFn = (path: string, opt?: Record<string, any>) => window.history.pushState(null, '', path),
		unsavedConfirmationModal
	}: Props = $props()

	migrateApp(app)

	const stateApp = $state(app)
	const appStore = writable<App>(stateApp)
	const selectedComponent = writable<string[] | undefined>(undefined)

	// $: selectedComponent.subscribe((s) => {
	// 	console.log('selectedComponent', s)
	// })
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
	let context = $state({
		email: $userStore?.email,
		groups: $userStore?.groups,
		username: $userStore?.username,
		name: $userStore?.name,
		query: Object.fromEntries(new URL(window.location.href).searchParams.entries()),
		hash: window.location.hash.substring(1),
		workspace: $workspaceStore,
		mode: 'editor',
		summary: $summaryStore,
		author: policy.on_behalf_of_email
	})
	const darkMode: Writable<boolean> = writable(document.documentElement.classList.contains('dark'))

	const worldStore = buildWorld(untrack(() => context))
	const previewTheme: Writable<string | undefined> = writable(undefined)
	const initialized = writable({
		initialized: false,
		initializedComponents: [],
		runnableInitialized: {}
	})
	const panzoomActive = writable(false)

	summaryStore.subscribe((s) => {
		$worldStore?.outputsById['ctx'].summary.set(s)
	})

	$secondaryMenuRightStore.isOpen = false
	$secondaryMenuLeftStore.isOpen = false

	let writablePath = writable(path)

	function onPathChange() {
		writablePath.set(path)
	}

	setContext<AppViewerContext>('AppViewerContext', {
		worldStore,
		app: appStore,
		summary: summaryStore,
		initialized: initialized,
		selectedComponent,
		mode,
		connectingInput,
		bgRuns: writable([]),
		breakpoint,
		runnableComponents: writable({}),
		appPath: writablePath,
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
		previewTheme,
		debuggingComponents: writable({}),
		replaceStateFn: replaceStateFn,
		policy: policy,
		recomputeAllContext: writable({
			loading: false,
			componentNumber: 0,
			refreshing: [],
			progress: 100
		}),
		panzoomActive
	})

	setContext<EditorPositionMap>(EDITOR_POSITION_MAP_CONTEXT_KEY, {})

	let scale = writable(100)

	const componentActive = writable(false)

	let yTop = writable(0)

	let runnableJob = writable({ focused: false, jobs: {}, frontendJobs: {}, width: 100 })
	setContext<AppEditorContext>('AppEditorContext', {
		yTop,
		runnableJobEditorPanel: runnableJob,
		evalPreview: writable({}),
		componentActive,
		dndItem: writable({}),
		refreshComponents: writable(undefined),
		history,
		pickVariableCallback,
		movingcomponents: writable(undefined),
		selectedComponentInEditor: writable(undefined),
		jobsDrawerOpen: writable(false),
		scale,
		stylePanel: () => StylePanel
	})

	let timeout: NodeJS.Timeout | undefined = undefined

	function saveFrontendDraft() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			try {
				localStorage.setItem(path != '' ? `app-${path}` : 'app', encodeState($appStore))
			} catch (err) {
				console.error('Error storing frontend draft in localStorage', err)
			}
		}, 500)
	}

	function hashchange(e: HashChangeEvent) {
		context.hash = e.newURL.split('#')[1]
		context = context
	}

	let selectedTab: 'insert' | 'settings' | 'css' = $state('insert')

	let befSelected: string | undefined = $state(undefined)

	function onSelectedComponentChange() {
		selectedTab = 'settings'
		if (befSelected) {
			if (!['ctx', 'state'].includes(befSelected) && !befSelected?.startsWith(BG_PREFIX)) {
				let item = findGridItem($appStore, befSelected)
				if (
					item?.data.type === 'containercomponent' ||
					item?.data.type === 'listcomponent' ||
					item?.data.type === 'modalcomponent'
				) {
					$focusedGrid = {
						parentComponentId: befSelected,
						subGridIndex: 0
					}
				} else if (item?.data.type === 'steppercomponent') {
					$focusedGrid = {
						parentComponentId: befSelected,
						subGridIndex:
							($worldStore.outputsById?.[befSelected]?.currentStepIndex?.peak() as number) ?? 0
					}
				} else if (
					item?.data.type === 'tabscomponent' ||
					item?.data.type === 'conditionalwrapper'
				) {
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
						const drawerAlreadyHandledFocusedGrid =
							item?.data.type === 'drawercomponent' &&
							$focusedGrid?.parentComponentId === befSelected
						if (!drawerAlreadyHandledFocusedGrid) {
							$focusedGrid = undefined
						}
					}
				}
			}
		}
	}

	let itemPicker: ItemPicker | undefined = $state(undefined)

	let variableEditor: VariableEditor | undefined = $state(undefined)

	setContext<ContextPanelContext>('ContextPanel', {
		search: writable<string>(''),
		manuallyOpened: writable<Record<string, boolean>>({}),
		hasResult: writable<Record<string, boolean>>({})
	})

	function onThemeChange() {
		$darkMode = document.documentElement.classList.contains('dark')
	}

	let runnablePanelSize = $state(30)
	let gridPanelSize = $state(70)

	let leftPanelSize = $state(22)
	let centerPanelSize = $state(56)
	let rightPanelSize = $state(22)

	let tmpRunnablePanelSize = -1
	let tmpGridPanelSize = -1

	let tmpLeftPanelSize = -1
	let tmpCenterPanelSize = -1
	let tmpRightPanelSize = -1

	let toggled = false
	let cssToggled = false

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

	function selectCss() {
		selectedTab !== 'css' && (selectedTab = 'css')
	}

	const cssId = 'wm-global-style'

	let css: string | undefined = $state(undefined)

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
				if (loadedCss) {
					css = loadedCss.value
				}
			}
			lastTheme = JSON.stringify(currentAppStore.theme)
		}
	})

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

	let appEditorHeader: AppEditorHeader | undefined = $state(undefined)

	export function triggerTutorial() {
		appEditorHeader?.toggleTutorial()
	}

	let box: HTMLElement | undefined = $state(undefined)
	function parseScroll() {
		$yTop = box?.scrollTop ?? 0
		// console.log('parse scroll', $yTop)
	}

	let mounted = false
	onMount(() => {
		mounted = true

		setTimeout(() => {
			if ($initialized?.initialized === false) {
				sendUserToast(
					'App is not yet initialized, please check the Troubleshoot Panel for more information',
					true,
					[
						{
							label: 'Open troubleshoot panel',
							callback: () => {
								appEditorHeader?.openTroubleshootPanel()
							}
						}
					]
				)
			}
		}, 10000)

		parseScroll()
	})

	let lastComponentActive = false

	$effect(() => {
		if ($componentActive != lastComponentActive) {
			if (box) {
				box.scrollTop = $yTop
			}
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

	let runnableJobEnterTimeout: NodeJS.Timeout | undefined = $state(undefined)
	let stillInJobEnter = $state(false)
	let storedLeftPanelSize = 0
	let storedRightPanelSize = 0
	let storedBottomPanelSize = 0

	let centerPanelWidth = $state(0)

	function hideLeftPanel(animate: boolean = false) {
		storedLeftPanelSize = leftPanelSize
		if (animate) {
			animateTo(leftPanelSize, 0, (newValue: number) => (leftPanelSize = newValue))
		} else {
			leftPanelSize = 0
		}
		centerPanelSize = centerPanelSize + storedLeftPanelSize
		if ($connectingInput.opened) {
			$connectingInput.opened = false
		}
	}

	function hideRightPanel() {
		storedRightPanelSize = rightPanelSize
		rightPanelSize = 0
		centerPanelSize = centerPanelSize + storedRightPanelSize
		if ($connectingInput.opened) {
			$connectingInput.opened = false
		}
	}

	function hideBottomPanel(animate: boolean = false) {
		if (runnablePanelSize === 0) {
			return
		}
		storedBottomPanelSize = runnablePanelSize
		if (animate) {
			tmpRunnablePanelSize = runnablePanelSize
			tmpGridPanelSize = gridPanelSize

			animateTo(runnablePanelSize, 0, (newValue: number) => (runnablePanelSize = newValue))
			animateTo(gridPanelSize, 100, (newValue: number) => (gridPanelSize = newValue))
		} else {
			runnablePanelSize = 0
			gridPanelSize = 99
		}
	}

	function showLeftPanel(animate: boolean = false) {
		if (leftPanelSize !== 0) {
			return
		}
		if (animate) {
			animateTo(
				leftPanelSize,
				storedLeftPanelSize,
				(newValue: number) => (leftPanelSize = newValue)
			)
		} else {
			leftPanelSize = storedLeftPanelSize
		}
	}

	function showRightPanel() {
		rightPanelSize = storedRightPanelSize
		centerPanelSize = centerPanelSize - storedRightPanelSize
	}

	function showBottomPanel(animate: boolean = false) {
		if (runnablePanelSize !== 0) {
			return
		}
		if (animate) {
			animateTo(
				runnablePanelSize,
				storedBottomPanelSize,
				(newValue: number) => (runnablePanelSize = newValue)
			)
			animateTo(
				gridPanelSize,
				gridPanelSize - storedBottomPanelSize,
				(newValue: number) => (gridPanelSize = newValue)
			)
		} else {
			runnablePanelSize = storedBottomPanelSize
			gridPanelSize = gridPanelSize - storedBottomPanelSize
		}
	}

	function keydown(event: KeyboardEvent) {
		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}

		isModifierKeyPressed = event.ctrlKey || event.metaKey

		switch (event.key) {
			case 'b': {
				if (event.ctrlKey || event.metaKey) {
					event.preventDefault()

					if (leftPanelSize !== 0) {
						hideLeftPanel()
					} else {
						showLeftPanel()
					}
				}
				break
			}

			case 'u': {
				if (event.ctrlKey || event.metaKey) {
					event.preventDefault()

					if (rightPanelSize !== 0) {
						hideRightPanel()
					} else {
						showRightPanel()
					}
				}
				break
			}

			case 'l': {
				if (event.ctrlKey || event.metaKey) {
					event.preventDefault()

					if (runnablePanelSize !== 0) {
						hideBottomPanel()
					} else {
						showBottomPanel()
					}
				}
				break
			}

			case 'Escape': {
				if ($connectingInput.opened) {
					$connectingInput.opened = false
				}
				if (handMode) {
					handMode = false
				}
				break
			}
		}
	}

	let previousLeftPanelHidden: boolean = false
	let previousBottomPanelHidden: boolean = false

	async function updatePannelInConnecting() {
		if ($connectingInput.opened && !toggled) {
			previousLeftPanelHidden = leftPanelSize === 0
			previousBottomPanelHidden = runnablePanelSize === 0
			if (previousLeftPanelHidden) {
				showLeftPanel(true)
			}
			if (!previousBottomPanelHidden) {
				hideBottomPanel(true)
			}
			secondaryMenuLeft.close()
			toggled = true
		} else if (!$connectingInput.opened && toggled) {
			if (previousLeftPanelHidden) {
				hideLeftPanel(true)
			}
			if (!previousBottomPanelHidden) {
				showBottomPanel(true)
			}
			toggled = false
		}
	}

	function updateCursorStyle(disabled: boolean) {
		if (disabled) {
			// Select all elements that don't have data-connection-button and aren't children of elements with data-connection-button
			const elements = document.querySelectorAll(
				':not([data-connection-button]):not([data-connection-button] *)'
			)
			elements.forEach((element) => {
				;(element as HTMLElement).style.cursor = 'not-allowed'
			})
		} else {
			// Reset cursor style for all elements
			const elements = document.querySelectorAll('*')
			elements.forEach((element) => {
				;(element as HTMLElement).style.removeProperty('cursor')
			})
		}
	}

	let instance: any
	let isModifierKeyPressed = $state(false)
	function resetView() {
		if (instance && box) {
			instance.moveTo(0.5, 0)
			instance.zoomAbs(0.5, 0, 1)
			box.scrollTop = 0
		}
	}

	function zoomTo(x: number, y: number, scale: number) {
		if (instance) {
			instance.zoomAbs(x, y, scale)
		}
	}

	function initPanzoom(node: HTMLElement) {
		instance = panzoom(node, {
			bounds: true,
			boundsPadding: 0.1,
			maxZoom: 1.5,
			minZoom: 0.3,
			zoomDoubleClickSpeed: 1,
			smoothScroll: false,
			initialZoom: $scale / 100.0,
			beforeMouseDown: (e) => {
				if (e.ctrlKey || e.metaKey || handMode) {
					// Prevent event propagation to children when panning
					e.stopPropagation()
					return false
				}
				return true
			},
			beforeWheel: (e) => {
				if (e.ctrlKey || e.metaKey || handMode) {
					// Prevent event propagation to children when zooming
					e.stopPropagation()
					return false
				}
				return true
			}
		})

		// Handle pointerdown when connecting
		node.addEventListener('pointerdown_connecting', (e) => {
			instance.handleDown(e)
		})

		// Update scale store when zoom changes
		instance.on('zoom', (e) => {
			const currentScale = e.getTransform().scale * 100
			if (currentScale !== $scale) {
				$scale = currentScale
			}
		})

		return {
			destroy() {
				instance.dispose()
				node.removeEventListener('pointerdown_connecting', instance.handleDown)
			}
		}
	}

	function handleKeyUp(e: KeyboardEvent) {
		isModifierKeyPressed = false
	}

	let mouseOverGridView = $state(false)
	let handMode = $state(false)
	let forceDeactivatePanzoom = $state(false)

	$effect(() => {
		path && untrack(() => onPathChange())
	})
	$effect(() => {
		$appStore && untrack(() => saveFrontendDraft())
	})
	$effect(() => {
		context.mode = $mode == 'dnd' ? 'editor' : 'viewer'
	})
	let width = $derived(
		$breakpoint === 'sm' && $appStore?.mobileViewOnSmallerScreens !== false
			? 'min-w-[400px] max-w-[656px]'
			: `min-w-[710px] ${$appStore.fullscreen ? 'w-full' : 'max-w-7xl'}`
	)
	$effect(() => {
		if ($selectedComponent?.[0] != befSelected) {
			befSelected = $selectedComponent?.[0]
			if ($selectedComponent?.[0] != undefined) {
				untrack(() => onSelectedComponentChange())
			}
		}
	})
	$effect(() => {
		if ($pickVariableCallback) {
			itemPicker?.openDrawer()
		}
	})
	// Animation logic for cssInput
	$effect(() => {
		;[$cssEditorOpen]
		untrack(() => animateCssInput($cssEditorOpen))
	})
	$effect(() => {
		$cssEditorOpen &&
			secondaryMenuLeft?.open(StylePanel, {
				type: 'style'
			})
	})
	$effect(() => {
		$cssEditorOpen && untrack(() => selectCss())
	})
	$effect(() => {
		;[$mode]
		untrack(() => addOrRemoveCss(true, $mode === 'preview'))
	})
	$effect(() => {
		;[css, $previewTheme]
		untrack(() => updateCssContent(css, $previewTheme))
	})
	$effect(() => {
		;[$componentActive]
		untrack(() => setGridPanelSize($componentActive))
	})
	$effect(() => {
		$connectingInput.opened, untrack(() => updatePannelInConnecting())
	})
	$effect(() => {
		forceDeactivatePanzoom = isModifierKeyPressed && handMode
	})
	$effect(() => {
		$panzoomActive =
			(isModifierKeyPressed || handMode) &&
			!forceDeactivatePanzoom &&
			!$componentActive &&
			mouseOverGridView
	})
	$effect(() => {
		;[!!$connectingInput.opened, !$panzoomActive]
		untrack(() => updateCursorStyle(!!$connectingInput.opened && !$panzoomActive))
	})

	const unsavedConfirmationModal_render = $derived(unsavedConfirmationModal)
</script>

<svelte:head></svelte:head>

<DarkModeObserver on:change={onThemeChange} />

<svelte:window
	onhashchange={hashchange}
	onkeydown={keydown}
	onkeyup={handleKeyUp}
	onfocus={() => {
		if (isModifierKeyPressed) {
			isModifierKeyPressed = false
		}
	}}
/>

<!-- {$focusedGrid?.parentComponentId} -->
{#if !$userStore?.operator}
	{#if $appStore}
		<AppEditorHeader
			{newPath}
			{newApp}
			on:restore
			{policy}
			{fromHub}
			bind:this={appEditorHeader}
			{diffDrawer}
			bind:savedApp
			{version}
			leftPanelHidden={leftPanelSize === 0}
			rightPanelHidden={rightPanelSize === 0}
			bottomPanelHidden={runnablePanelSize === 0}
			on:savedNewAppPath
			on:showLeftPanel={() => showLeftPanel()}
			on:showRightPanel={() => showRightPanel()}
			on:hideLeftPanel={() => hideLeftPanel()}
			on:hideRightPanel={() => hideRightPanel()}
			on:hideBottomPanel={() => hideBottomPanel()}
			on:showBottomPanel={() => showBottomPanel()}
		>
			{#snippet unsavedConfirmationModal({
				diffDrawer,
				additionalExitAction,
				getInitialAndModifiedValues
			})}
				{@render unsavedConfirmationModal_render?.({
					diffDrawer,
					additionalExitAction,
					getInitialAndModifiedValues
				})}
			{/snippet}
		</AppEditorHeader>
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
						{replaceStateFn}
						{gotoFn}
					/>
				</div>
			</SplitPanesWrapper>
		{:else}
			{#if $componentActive}
				<div
					class="absolute z-50 inset-0 h-full w-full bg-surface-secondary [background-size:16px_16px]"
				>
					<div class="w-min whitespace-nowrap mx-auto pt-0.5 z-50">
						<Alert
							title={`Press ${getModifierKey()} to drop component inside a container.`}
							size="xs"
							class="h-10 py-1"
						/>
					</div>
				</div>
			{/if}

			{#if $connectingInput.opened}
				<div class="absolute z-50 inset-0 w-min h-min whitespace-nowrap mx-auto pt-0.5">
					<Alert title="Press Esc to exit connection mode." size="xs" class="h-10 py-1" />
				</div>
			{/if}

			<SplitPanesWrapper>
				<Splitpanes id="o1" class="max-w-full overflow-hidden">
					<Pane bind:size={leftPanelSize} minSize={5} maxSize={33}>
						<div
							class={twMerge(
								'w-full h-full relative',
								$secondaryMenuLeftStore.isOpen ? 'overflow-hidden' : ''
							)}
						>
							<!-- {yTop} -->

							<SecondaryMenu right={false} />
							<ContextPanel on:hidePanel={() => hideLeftPanel()} />
						</div>
					</Pane>
					<Pane bind:size={centerPanelSize}>
						<Splitpanes id="o2" horizontal class="!overflow-visible">
							<Pane bind:size={gridPanelSize} class="ovisible">
								<div
									onpointerdown={(e) => {
										$selectedComponent = undefined
										$focusedGrid = undefined
									}}
									class={twMerge(
										'bg-surface-secondary h-full w-full relative',
										$appStore.css?.['app']?.['viewer']?.class,
										'wm-app-viewer h-full overflow-visible',
										$panzoomActive ? 'cursor-grab' : ''
									)}
									style={$appStore.css?.['app']?.['viewer']?.style}
									bind:clientWidth={centerPanelWidth}
								>
									{#if leftPanelSize === 0}
										<div class="absolute top-0.5 left-0.5 z-50">
											<HideButton
												on:click={() => showLeftPanel()}
												direction="left"
												hidden
												btnClasses="border bg-surface"
											/>
										</div>
									{/if}
									{#if rightPanelSize === 0}
										<div class="absolute top-0.5 right-0.5 z-50">
											<HideButton
												on:click={() => showRightPanel()}
												direction="right"
												hidden
												btnClasses="border bg-surface"
											/>
										</div>
									{/if}
									{#if runnablePanelSize === 0}
										<div class="absolute bottom-0.5 right-0.5 z-50">
											<HideButton
												on:click={() => showBottomPanel()}
												direction="bottom"
												hidden
												btnClasses="border bg-surface"
											/>
										</div>
									{/if}

									<div
										class="absolute bottom-2 left-2 z-50 border bg-surface"
										data-connection-button
										onpointerdown={stopPropagation(bubble('pointerdown'))}
									>
										<div class="flex flex-row gap-2 text-xs items-center h-8 px-1">
											<Button
												color="light"
												size="xs2"
												disabled={$scale <= 30}
												on:click={() => {
													zoomTo(0.5, 0, Math.round(($scale - 10) / 10) / 10)
												}}
											>
												<Minus size={14} />
											</Button>
											<div class="w-8 flex justify-center text-2xs h-full items-center">
												{Math.round($scale)}%
											</div>
											<Button
												color="light"
												size="xs2"
												disabled={$scale >= 150}
												on:click={() => {
													zoomTo(0.5, 0, Math.round(($scale + 10) / 10) / 10)
												}}
											>
												<Plus size={14} />
											</Button>
											<Button color="light" size="xs2" disabled={false} on:click={resetView}>
												<Scan size={14} />
											</Button>
											<Popover disappearTimeout={0} notClickable placement="bottom">
												{#snippet text()}
													<div class="flex flex-row gap-2">
														<div class="flex-1">
															Hand Mode
															<br />
															<span class="ml-2">Pan</span>
															<br />
															<span class="ml-2">Zoom</span>

															<br />
															<span class="ml-2">Exit</span>
														</div>
														<div>
															<span class="float-left text-tertiary-inverse"
																>hold {getModifierKey()}</span
															>
															<br />
															<span class="float-left text-tertiary-inverse">click & drag</span>
															<br />
															<span class="float-left text-tertiary-inverse">scroll</span>
															<br />
															<span class="float-left text-tertiary-inverse">esc</span>
														</div>
													</div>
												{/snippet}
												<Button
													color="light"
													size="xs2"
													disabled={false}
													on:click={() => (handMode = !handMode)}
													btnClasses={handMode ? 'bg-surface-hover' : ''}
												>
													{#if $panzoomActive}
														<Grab size={14} />
													{:else}
														<Hand size={14} />
													{/if}
												</Button>
											</Popover>
										</div>
									</div>

									<div id="app-editor-top-level-drawer"></div>
									<div
										class="absolute pointer-events-none inset-0 h-full w-full surface-secondary bg-[radial-gradient(#dbdbdb_1px,transparent_1px)] dark:bg-[radial-gradient(#666666_1px,transparent_1px)] [background-size:16px_16px]"
									></div>

									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div
										bind:this={box}
										onscroll={parseScroll}
										onmouseenter={() => {
											mouseOverGridView = true
										}}
										onmouseleave={() => {
											mouseOverGridView = false
										}}
										class={classNames(
											'mx-auto w-full h-full z-50',
											$componentActive ? 'absolute right-0 left-0' : 'overflow-auto'
										)}
										style={$componentActive ? `top: -${$yTop}px;` : ''}
									>
										{#if $appStore.grid}
											{#if !$connectingInput?.opened}
												<ComponentNavigation />
											{/if}

											<div
												class={twMerge(
													width,
													'mx-auto',
													'z-10000',
													$panzoomActive ? 'pointer-events-none' : ''
												)}
												use:initPanzoom
											>
												<GridEditor {policy} />
											</div>
										{/if}
										{#if !$appStore?.mobileViewOnSmallerScreens && $breakpoint === 'sm'}
											<div
												class="absolute inset-0 flex bg-surface center-center z-10000 bg-opacity-60"
											>
												<div
													class="bg-surface shadow-md rounded-md p-4 max-w-sm flex flex-col gap-2"
												>
													<div class="text-sm font-semibold"> Mobile View </div>
													<div class="text-xs">
														Enabling mobile view allows you to adjust component placement for
														smaller screens.
													</div>
													<Button
														color="light"
														variant="border"
														size="xs"
														on:click={() => {
															$appStore.mobileViewOnSmallerScreens = true
														}}
														startIcon={{
															icon: Smartphone
														}}
													>
														Add mobile view on smaller screens
													</Button>
												</div>
											</div>
										{/if}
									</div>
								</div>
							</Pane>
							{#if $connectingInput?.opened == false && !$componentActive}
								<Pane bind:size={runnablePanelSize}>
									<AppEditorBottomPanel
										on:mouseenter={() => {
											runnableJobEnterTimeout && clearTimeout(runnableJobEnterTimeout)
											stillInJobEnter = true
											runnableJobEnterTimeout = setTimeout(() => {
												if (stillInJobEnter) {
													$runnableJob.focused = true
												}
											}, 200)
										}}
										on:hidePanel={() => hideBottomPanel()}
										on:mouseleave={() => {
											stillInJobEnter = false
											runnableJobEnterTimeout = setTimeout(
												() => ($runnableJob.focused = false),
												200
											)
										}}
										{rightPanelSize}
										{centerPanelWidth}
										{runnablePanelSize}
									/>
								</Pane>
							{/if}
						</Splitpanes>
					</Pane>
					{#if rightPanelSize === 0}
						<div class="relative flex flex-col h-full"></div>
					{:else}
						<Pane bind:size={rightPanelSize} minSize={15} maxSize={33}>
							<div bind:clientWidth={$runnableJob.width} class="relative flex flex-col h-full">
								<Tabs bind:selected={selectedTab} wrapperClass="!min-h-[42px]" class="!h-full">
									<Popover disappearTimeout={0} notClickable placement="bottom">
										{#snippet text()}
											Component library
										{/snippet}
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
										{#snippet text()}
											Component settings
										{/snippet}
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
										{#snippet text()}
											Global styling
										{/snippet}
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
									<div class="h-full w-full flex justify-end px-1">
										<HideButton on:click={() => hideRightPanel()} direction="right" />
									</div>
									{#snippet content()}
										<div class="h-full overflow-y-auto">
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
														Select a component to see the settings for it
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
									{/snippet}
								</Tabs>
							</div>
						</Pane>
					{/if}
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
	{#snippet submission()}
		<div
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
				New variable
			</Button>
		</div>
	{/snippet}
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
