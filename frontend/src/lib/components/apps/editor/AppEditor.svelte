<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { onMount, setContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import { buildWorld } from '../rx'
	import type {
		App,
		AppEditorContext,
		AppViewerContext,
		ConnectingInput,
		EditorBreakpoint,
		EditorMode,
		FocusedGrid,
		InlineScript
	} from '../types'
	import AppEditorHeader from './AppEditorHeader.svelte'
	import GridEditor from './GridEditor.svelte'

	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import { Alert, Button, Tab } from '$lib/components/common'
	import ComponentList from './componentsPanel/ComponentList.svelte'
	import Icon from 'svelte-awesome'
	import { faCode, faPlus, faSliders } from '@fortawesome/free-solid-svg-icons'
	import ContextPanel from './contextPanel/ContextPanel.svelte'
	import { classNames, encodeState } from '$lib/utils'
	import AppPreview from './AppPreview.svelte'
	import { userStore, workspaceStore } from '$lib/stores'

	import InlineScriptsPanel from './inlineScriptsPanel/InlineScriptsPanel.svelte'

	import SettingsPanel from './SettingsPanel.svelte'
	import { fly } from 'svelte/transition'
	import { VariableService, type Policy } from '$lib/gen'
	import { page } from '$app/stores'
	import CssSettings from './componentsPanel/CssSettings.svelte'
	import { initHistory } from '$lib/history'
	import ComponentNavigation from './component/ComponentNavigation.svelte'
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import VariableEditor from '$lib/components/VariableEditor.svelte'

	export let app: App
	export let path: string
	export let initialMode: EditorMode = 'dnd'
	export let policy: Policy
	export let summary: string
	export let fromHub: boolean = false

	const appStore = writable<App>(app)
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>(initialMode)
	const breakpoint = writable<EditorBreakpoint>('lg')
	const summaryStore = writable(summary)
	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined,
		hoveredComponent: undefined
	})
	const history = initHistory(app)

	const runnableComponents = writable<
		Record<string, (inlineScript?: InlineScript) => Promise<void>>
	>({})
	const errorByComponent = writable<Record<string, { error: string; componentId: string }>>({})
	const focusedGrid = writable<FocusedGrid | undefined>(undefined)
	const pickVariableCallback: Writable<((path: string) => void) | undefined> = writable(undefined)
	let context = {
		email: $userStore?.email,
		username: $userStore?.username,
		query: Object.fromEntries($page.url.searchParams.entries()),
		hash: $page.url.hash
	}

	setContext<AppViewerContext>('AppViewerContext', {
		worldStore: buildWorld(context),
		app: appStore,
		summary: summaryStore,
		selectedComponent,
		mode,
		connectingInput,
		breakpoint,
		runnableComponents,
		appPath: path,
		workspace: $workspaceStore ?? '',
		onchange: () => saveDraft(),
		isEditor: true,
		jobs: writable([]),
		staticExporter: writable({}),
		noBackend: false,
		errorByComponent,
		openDebugRun: writable(undefined),
		focusedGrid,
		stateId: writable(0),
		parentWidth: writable(0),
		state: writable({}),
		componentControl: writable({}),
		hoverStore: writable(undefined)
	})

	setContext<AppEditorContext>('AppEditorContext', {
		history,
		pickVariableCallback,
		ontextfocus: writable(undefined),
		movingcomponent: writable(undefined),
		selectedComponentInEditor: writable(undefined)
	})

	let timeout: NodeJS.Timeout | undefined = undefined

	$: $appStore && saveDraft()

	function saveDraft() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			try {
				localStorage.setItem(path != '' ? `app-${path}` : 'app', encodeState($appStore))
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	let mounted = false

	onMount(() => {
		mounted = true
	})

	function hashchange(e: HashChangeEvent) {
		context.hash = e.newURL.split('#')[1]
		context = context
	}

	$: previewing = $mode === 'preview'
	$: width = $breakpoint === 'sm' ? 'min-w-[400px] max-w-[656px]' : 'min-w-[710px] w-full'

	let selectedTab: 'insert' | 'settings' = 'insert'

	$: if ($selectedComponent) {
		selectedTab = 'settings'
	} else {
		selectedTab = 'insert'
	}

	let itemPicker: ItemPicker | undefined = undefined

	$: if ($pickVariableCallback) {
		itemPicker?.openDrawer()
	}

	let variableEditor: VariableEditor | undefined = undefined
</script>

<svelte:window on:hashchange={hashchange} />

{#if $connectingInput.opened}
	<div
		class="absolute w-full h-screen bg-black border-2 bg-opacity-25 z-20 flex justify-center items-center"
	/>
{/if}
{#if !$userStore?.operator}
	{#if $appStore}
		{#if initialMode !== 'preview'}
			<AppEditorHeader {policy} {fromHub} />
		{/if}

		{#if previewing}
			<SplitPanesWrapper>
				<div
					class={twMerge('h-full w-full', $appStore.css?.['app']?.['viewer']?.class)}
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
			<SplitPanesWrapper>
				<Splitpanes class="max-w-full overflow-hidden">
					<Pane size={15} minSize={5} maxSize={33}>
						<ContextPanel />
					</Pane>
					<Pane size={64}>
						<SplitPanesWrapper>
							<Splitpanes horizontal>
								<Pane size={70}>
									<div
										on:pointerdown={(e) => {
											$selectedComponent = undefined
											$focusedGrid = undefined
										}}
										class={twMerge(
											'bg-gray-100 h-full w-full',
											$appStore.css?.['app']?.['viewer']?.class
										)}
										style={$appStore.css?.['app']?.['viewer']?.style}
									>
										<div
											class={classNames(
												'relative mx-auto w-full h-full overflow-auto',
												app.fullscreen ? '' : 'max-w-6xl'
											)}
										>
											{#if $appStore.grid}
												<ComponentNavigation />

												<div on:pointerdown|stopPropagation class={width}>
													<GridEditor {policy} />
												</div>

												<div id="app-editor-top-level-drawer" />
											{/if}
										</div>
									</div>
								</Pane>
								<Pane size={$connectingInput?.opened ? 0 : 30}>
									<div class="relative h-full w-full">
										<InlineScriptsPanel />
									</div>
								</Pane>
							</Splitpanes>
						</SplitPanesWrapper>
					</Pane>
					<Pane size={21} minSize={5} maxSize={33}>
						<div class="relative flex flex-col h-full">
							<Tabs bind:selected={selectedTab} wrapperClass="!h-[40px]" class="!h-full">
								<Tab value="insert" size="xs">
									<div class="m-1 center-center gap-2">
										<Icon data={faPlus} />
										<span>Insert</span>
									</div>
								</Tab>
								<Tab value="settings" size="xs">
									<div class="m-1 center-center gap-2">
										<Icon data={faSliders} />
										<span>Settings</span>
									</div>
								</Tab>
								<Tab value="css" size="xs">
									<div class="m-1 center-center gap-2">
										<Icon data={faCode} />
										<span>CSS</span>
									</div>
								</Tab>
								<div slot="content" class="h-full overflow-y-auto">
									<TabContent class="overflow-auto h-full" value="settings">
										{#if $selectedComponent !== undefined}
											{#key $selectedComponent}
												<SettingsPanel />
											{/key}
										{:else}
											<div class="min-w-[150px] text-sm text-gray-500 text-center py-8 px-2">
												Select a component to see the settings&nbsp;for&nbsp;it
											</div>
										{/if}
									</TabContent>
									<TabContent value="insert">
										<ComponentList />
									</TabContent>
									<TabContent value="css">
										<CssSettings />
									</TabContent>
								</div>
							</Tabs>
							{#if $connectingInput.opened}
								<div
									class="fixed top-32  p-2 z-50 flex justify-center items-center"
									transition:fly|local={{ duration: 100, y: -100 }}
								>
									<Alert title="Connecting" type="info">
										<div class="flex gap-2 flex-col">
											Click on the output of the component you want to connect to on the left panel.
											<div>
												<Button
													color="blue"
													variant="border"
													size="xs"
													on:click={() => {
														$connectingInput.opened = false
														$connectingInput.input = undefined
													}}
												>
													Stop connecting
												</Button>
											</div>
										</div>
									</Alert>
								</div>
							{/if}
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
		class="flex flex-row-reverse w-full bg-white border-t border-gray-200 rounded-bl-lg rounded-br-lg"
	>
		<Button
			variant="border"
			color="blue"
			size="sm"
			startIcon={{ icon: faPlus }}
			on:click={() => {
				variableEditor?.initNew?.()
			}}
		>
			New variable
		</Button>
	</div>
</ItemPicker>

<VariableEditor bind:this={variableEditor} />
