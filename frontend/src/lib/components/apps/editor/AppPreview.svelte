<script lang="ts">
	import { run } from 'svelte/legacy'

	import { getContext, onDestroy, setContext } from 'svelte'
	import { get, writable, type Writable } from 'svelte/store'
	import { buildWorld } from '../rx'
	import type {
		App,
		AppViewerContext,
		ConnectingInput,
		EditorBreakpoint,
		EditorMode
	} from '../types'
	import type { Policy } from '$lib/gen'
	import Button from '../../common/button/Button.svelte'
	import { Unlock } from 'lucide-svelte'
	import GridViewer from './GridViewer.svelte'
	import Component from './component/Component.svelte'
	import { twMerge } from 'tailwind-merge'
	import { deepEqual } from 'fast-equals'
	import { dfs, maxHeight } from './appUtils'
	import { BG_PREFIX, migrateApp } from '../utils'
	import { workspaceStore, enterpriseLicense } from '$lib/stores'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { getTheme } from './componentsPanel/themeUtils'
	import HiddenComponent from '../components/helpers/HiddenComponent.svelte'
	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import type { StateStore } from '$lib/utils'

	interface Props {
		app: App
		appPath?: string
		breakpoint?: Writable<EditorBreakpoint>
		policy?: Policy
		summary?: string
		workspace?: string
		isEditor?: boolean
		context: Record<string, any>
		noBackend?: boolean
		isLocked?: boolean
		hideRefreshBar?: boolean
		class?: string
		replaceStateFn?: (path: string) => void
		gotoFn?: (path: string, opt?: Record<string, any> | undefined) => void
	}

	let {
		app,
		appPath = '',
		breakpoint = writable('lg'),
		policy = {},
		summary = '',
		workspace = $workspaceStore!,
		isEditor = false,
		context,
		noBackend = false,
		isLocked = $bindable(false),
		hideRefreshBar = false,
		class: className = '',
		replaceStateFn = (path: string) => window.history.replaceState(null, '', path),
		gotoFn = (path: string, opt?: Record<string, any>) => window.history.pushState(null, '', path)
	}: Props = $props()

	migrateApp(app)

	const appStore: StateStore<App> = $state({ val: app })
	const selectedComponent = writable<string[] | undefined>(undefined)
	const mode = writable<EditorMode>('preview')

	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined,
		hoveredComponent: undefined
	})

	const allIdsInPath = writable<string[]>([])

	let ncontext: any = {
		...context,
		workspace,
		mode: 'viewer',
		summary: summary,
		author: policy.on_behalf_of_email
	}

	function resizeWindow() {
		!isEditor && ($breakpoint = window.innerWidth < 769 ? 'sm' : 'lg')
	}

	resizeWindow()

	const parentWidth = writable(0)

	let previousDarkMode = document.documentElement.classList.contains('dark')
	const darkMode: Writable<boolean> = writable(app?.darkMode ?? previousDarkMode)

	onDestroy(() => {
		setTheme(previousDarkMode)
	})

	function setTheme(darkMode: boolean | undefined) {
		let globalDarkMode = window.localStorage.getItem('dark-mode')
			? window.localStorage.getItem('dark-mode') === 'dark'
			: window.matchMedia('(prefers-color-scheme: dark)').matches
		if (darkMode === true || (darkMode === null && globalDarkMode)) {
			document.documentElement.classList.add('dark')
		} else if (darkMode === false) {
			document.documentElement.classList.remove('dark')
		}
	}

	setTheme($darkMode)

	const state_ = writable({})

	let parentContext = getContext<AppViewerContext>('AppViewerContext')

	let worldStore = buildWorld(ncontext)

	function onContextChange(context: any) {
		Object.assign(ncontext, context)
		ncontext = ncontext
		Object.entries(context).forEach(([key, value]) => {
			get(worldStore).outputsById?.['ctx']?.[key].set(value, true)
		})
	}

	function hashchange(e: HashChangeEvent) {
		ncontext.hash = e.newURL.split('#')[1]
		ncontext = ncontext
		worldStore.update((x) => {
			x.outputsById?.['ctx']?.['hash'].set(ncontext.hash, true)
			return x
		})
	}

	let writablePath = writable(appPath)

	function onPathChange() {
		writablePath.set(appPath)
	}

	setContext<AppViewerContext>('AppViewerContext', {
		worldStore: worldStore,
		initialized: writable({
			initialized: false,
			initializedComponents: [],
			runnableInitialized: {}
		}),
		app: appStore,
		summary: writable(summary),
		selectedComponent,
		bgRuns: writable([]),
		mode,
		connectingInput,
		breakpoint,
		runnableComponents: writable({}),
		appPath: writablePath,
		workspace,
		onchange: undefined,
		isEditor,
		jobs: parentContext?.jobs ?? writable([]),
		jobsById: parentContext?.jobsById ?? writable({}),
		staticExporter: writable({}),
		noBackend,
		errorByComponent: writable({}),
		openDebugRun: writable(undefined),
		focusedGrid: writable(undefined),
		stateId: writable(0),
		parentWidth,
		state: state_,
		componentControl: writable({}),
		hoverStore: writable(undefined),
		allIdsInPath,
		darkMode,
		cssEditorOpen: writable(false),
		previewTheme: writable(undefined),
		debuggingComponents: writable({}),
		replaceStateFn,
		gotoFn,
		policy,
		recomputeAllContext: writable({
			loading: false,
			componentNumber: 0,
			refreshing: [],
			progress: 100
		}),
		panzoomActive: writable(false)
	})

	let previousSelectedIds: string[] | undefined = $state(undefined)

	function onThemeChange() {
		$darkMode = app?.darkMode ?? document.documentElement.classList.contains('dark')
	}

	const cssId = 'wm-global-style'

	let css: string | undefined = $state(undefined)

	$effect(() => {
		loadTheme(appStore.val)
	})

	async function loadTheme(currentAppStore: App) {
		if (!currentAppStore.theme) {
			return
		}

		if (currentAppStore.theme.type === 'inlined') {
			css = currentAppStore.theme.css
		} else if (currentAppStore.theme.type === 'path' && currentAppStore.theme.path) {
			let loadedCss = await getTheme(workspace, currentAppStore.theme.path)
			if (loadedCss) {
				css = loadedCss.value
			}
		}
	}

	function addOrRemoveCss(isPremium: boolean, cssString: string | undefined) {
		const existingElement = document.getElementById(cssId)

		if (!isPremium) {
			if (existingElement) {
				existingElement.remove()
			}
		} else {
			if (!existingElement && cssString) {
				const head = document.head
				const link = document.createElement('style')
				link.id = cssId
				link.innerHTML = cssString
				head.appendChild(link)
			} else if (existingElement) {
				if (cssString) {
					existingElement.innerHTML = cssString
				} else {
					existingElement.innerHTML = ''
				}
			}
		}
	}

	let appHeight: number = $state(0)

	run(() => {
		onContextChange(context)
	})
	run(() => {
		appPath && onPathChange()
	})
	run(() => {
		if (!deepEqual(previousSelectedIds, $selectedComponent)) {
			previousSelectedIds = $selectedComponent
			$allIdsInPath = ($selectedComponent ?? [])
				.flatMap((id) => dfs(app.grid, id, app.subgrids ?? {}))
				.filter((x) => x != undefined) as string[]
		}
	})
	let width = $derived(
		$breakpoint === 'sm' && appStore.val?.mobileViewOnSmallerScreens !== false
			? 'max-w-[640px]'
			: 'w-full min-w-[768px]'
	)
	let lockedClasses = $derived(isLocked ? '!max-h-[400px] overflow-hidden pointer-events-none' : '')
	run(() => {
		addOrRemoveCss($enterpriseLicense !== undefined || isEditor, css)
	})
	let maxRow = $derived(maxHeight(appStore.val.grid, appHeight, $breakpoint))
</script>

<svelte:head></svelte:head>

<DarkModeObserver on:change={onThemeChange} />

<svelte:window onhashchange={hashchange} onresize={resizeWindow} />

<div class="relative min-h-full grow" bind:clientHeight={appHeight}>
	<div id="app-editor-top-level-drawer"></div>
	<div id="app-editor-select"></div>

	<div
		class="{className} {lockedClasses} {width} h-full bg-surface {app.fullscreen
			? ''
			: 'max-w-7xl'} mx-auto"
		id="app-content"
	>
		{#if appStore.val.grid}
			<div
				class={twMerge(
					'mx-auto',
					hideRefreshBar || appStore.val?.norefreshbar || appStore.val.hideLegacyTopBar === true
						? 'invisible h-0 overflow-hidden'
						: ''
				)}
			>
				<div
					class="w-full sticky top-0 flex justify-between border-b bg-surface-secondary px-4 py-1 items-center gap-4"
				>
					<h2 class="truncate">{summary}</h2>
					<RecomputeAllComponents />
					<div class="text-2xs text-secondary">
						{policy.on_behalf_of ? `on behalf of ${policy.on_behalf_of_email}` : ''}
					</div>
				</div>
			</div>
		{/if}

		<div
			style={app.css?.['app']?.['grid']?.style}
			class={twMerge(
				'p-2 overflow-visible',
				app.css?.['app']?.['grid']?.class ?? '',
				'wm-app-grid subgrid'
			)}
			bind:clientWidth={$parentWidth}
		>
			<div>
				<GridViewer allIdsInPath={$allIdsInPath} items={app.grid} {maxRow} breakpoint={$breakpoint}>
					{#snippet children({ dataItem })}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div
							class={'h-full w-full center-center'}
							onpointerdown={() => ($selectedComponent = [dataItem.id])}
						>
							<Component
								render={true}
								component={dataItem.data}
								selected={false}
								locked={true}
								fullHeight={dataItem?.[$breakpoint === 'sm' ? 3 : 12]?.fullHeight}
							/>
						</div>
					{/snippet}
				</GridViewer>
			</div>
		</div>
	</div>

	{#if isLocked}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			onclick={() => (isLocked = false)}
			class="absolute inset-0 center-center bg-black/20 z-50 backdrop-blur-[1px] cursor-pointer"
		>
			<Button on:click={() => (isLocked = false)}>
				Unlock preview
				<Unlock size={18} class="ml-1" strokeWidth={2.5} />
			</Button>
		</div>
	{/if}
</div>

{#if app.hiddenInlineScripts}
	{#each app.hiddenInlineScripts as runnable, index}
		{#if runnable}
			<HiddenComponent id={BG_PREFIX + index} {runnable} />
		{/if}
	{/each}
{/if}
