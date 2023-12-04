<script lang="ts">
	import { getContext, setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import { buildWorld } from '../rx'
	import type {
		App,
		AppViewerContext,
		ConnectingInput,
		EditorBreakpoint,
		EditorMode
	} from '../types'
	import { classNames } from '$lib/utils'
	import type { Policy } from '$lib/gen'
	import Button from '../../common/button/Button.svelte'
	import { Unlock } from 'lucide-svelte'
	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import GridViewer from './GridViewer.svelte'
	import Component from './component/Component.svelte'
	import { twMerge } from 'tailwind-merge'
	import { columnConfiguration } from '../gridUtils'
	import { HiddenComponent } from '../components'
	import { deepEqual } from 'fast-equals'
	import { dfs } from './appUtils'
	import { BG_PREFIX, migrateApp } from '../utils'
	import { workspaceStore, enterpriseLicense } from '$lib/stores'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { getTheme } from './componentsPanel/themeUtils'

	export let app: App
	export let appPath: string = ''
	export let breakpoint: Writable<EditorBreakpoint> = writable('lg')
	export let policy: Policy = {}
	export let summary: string = ''
	export let workspace: string = $workspaceStore!
	export let isEditor: boolean = false
	export let context: Record<string, any>
	export let noBackend: boolean = false
	export let isLocked = false

	migrateApp(app)

	const appStore = writable<App>(app)
	const selectedComponent = writable<string[] | undefined>(undefined)
	const mode = writable<EditorMode>('preview')

	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined,
		hoveredComponent: undefined
	})

	const allIdsInPath = writable<string[]>([])

	let ncontext: any = { ...context, workspace, mode: 'viewer' }

	function hashchange(e: HashChangeEvent) {
		ncontext.hash = e.newURL.split('#')[1]
		ncontext = ncontext
	}

	function resizeWindow() {
		!isEditor && ($breakpoint = window.innerWidth < 769 ? 'sm' : 'lg')
	}

	resizeWindow()

	const parentWidth = writable(0)
	const darkMode: Writable<boolean> = writable(document.documentElement.classList.contains('dark'))

	const state = writable({})

	let parentContext = getContext<AppViewerContext>('AppViewerContext')

	setContext<AppViewerContext>('AppViewerContext', {
		worldStore: buildWorld(ncontext),
		initialized: writable({ initialized: false, initializedComponents: [] }),
		app: appStore,
		summary: writable(summary),
		selectedComponent,
		bgRuns: writable([]),
		mode,
		connectingInput,
		breakpoint,
		runnableComponents: writable({}),
		appPath,
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
		state: state,
		componentControl: writable({}),
		hoverStore: writable(undefined),
		allIdsInPath,
		darkMode,
		cssEditorOpen: writable(false),
		previewTheme: writable(undefined)
	})

	let previousSelectedIds: string[] | undefined = undefined
	$: if (!deepEqual(previousSelectedIds, $selectedComponent)) {
		previousSelectedIds = $selectedComponent
		$allIdsInPath = ($selectedComponent ?? [])
			.flatMap((id) => dfs(app.grid, id, app.subgrids ?? {}))
			.filter((x) => x != undefined) as string[]
	}

	$: width = $breakpoint === 'sm' ? 'max-w-[640px]' : 'w-full min-w-[768px]'
	$: lockedClasses = isLocked ? '!max-h-[400px] overflow-hidden pointer-events-none' : ''
	function onThemeChange() {
		$darkMode = document.documentElement.classList.contains('dark')
	}
	const cssId = 'wm-global-style'

	let css: string | undefined = undefined

	appStore.subscribe(loadTheme)

	async function loadTheme(currentAppStore: App) {
		if (!currentAppStore.theme) {
			return
		}

		if (currentAppStore.theme.type === 'inlined') {
			css = currentAppStore.theme.css
		} else if (currentAppStore.theme.type === 'path' && currentAppStore.theme.path) {
			let loadedCss = await getTheme(workspace, currentAppStore.theme.path)
			css = loadedCss.value
		}
	}

	$: addOrRemoveCss($enterpriseLicense !== undefined || isEditor, css)

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
			} else if (existingElement && cssString) {
				existingElement.innerHTML = cssString
			}
		}
	}
</script>

<DarkModeObserver on:change={onThemeChange} />

<svelte:window on:hashchange={hashchange} on:resize={resizeWindow} />

<div class="relative h-full">
	<div id="app-editor-top-level-drawer" />
	<div id="app-editor-select" />

	<div
		class="{$$props.class} {lockedClasses} {width} h-full bg-surface {app.fullscreen
			? ''
			: 'max-w-7xl'} mx-auto"
		id="app-content"
	>
		{#if $appStore.grid}
			<div
				class={classNames(
					'mx-auto',
					$appStore?.norefreshbar ? 'invisible h-0 overflow-hidden' : ''
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
				'wm-app-grid  subgrid'
			)}
			bind:clientWidth={$parentWidth}
		>
			<div>
				<GridViewer
					allIdsInPath={$allIdsInPath}
					items={app.grid}
					let:dataItem
					rowHeight={36}
					cols={columnConfiguration}
					gap={[4, 2]}
				>
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						class={'h-full w-full center-center'}
						on:pointerdown={() => ($selectedComponent = [dataItem.id])}
					>
						<Component render={true} component={dataItem.data} selected={false} locked={true} />
					</div>
				</GridViewer>
			</div>
		</div>
	</div>

	{#if isLocked}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div
			on:click={() => (isLocked = false)}
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
