<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { onMount, setContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type {
		App,
		AppEditorContext,
		ConnectingInput,
		EditorBreakpoint,
		EditorMode,
		FocusedGrid
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
	import type { Policy } from '$lib/gen'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { page } from '$app/stores'
	import CssSettings from './componentsPanel/CssSettings.svelte'

	export let app: App
	export let path: string
	export let initialMode: EditorMode = 'dnd'
	export let policy: Policy
	export let summary: string
	export let fromHub: boolean = false

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>(initialMode)
	const breakpoint = writable<EditorBreakpoint>('lg')
	const summaryStore = writable(summary)
	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined,
		hoveredComponent: undefined
	})

	const runnableComponents = writable<Record<string, () => Promise<void>>>({})
	const errorByComponent = writable<Record<string, { error: string; componentId: string }>>({})
	const focusedGrid = writable<FocusedGrid | undefined>(undefined)

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
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
		parentWidth: writable(0)
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

	$: context = {
		email: $userStore?.email,
		username: $userStore?.username,
		query: Object.fromEntries($page.url.searchParams.entries())
	}

	$: mounted && ($worldStore = buildWorld($staticOutputs, $worldStore, context))
	$: previewing = $mode === 'preview'
	$: width = $breakpoint === 'sm' ? 'min-w-[400px] max-w-[656px]' : 'min-w-[710px] w-full'

	let selectedTab: 'insert' | 'settings' = 'insert'
	$: if ($selectedComponent) {
		selectedTab = 'settings'
	} else {
		selectedTab = 'insert'
	}
</script>

{#if $connectingInput.opened}
	<div
		class="absolute w-full h-screen bg-black border-2 bg-opacity-25 z-20 flex justify-center items-center"
	/>
{/if}
{#if !$userStore?.operator}
	<UnsavedConfirmationModal />
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
				<Pane size={$connectingInput?.opened ? 40 : 15} minSize={5} maxSize={33}>
					<ContextPanel />
				</Pane>
				<Pane size={64}>
					<SplitPanesWrapper>
						<Splitpanes horizontal>
							<Pane size={$connectingInput?.opened ? 100 : 70}>
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
						<Tabs bind:selected={selectedTab} class="!border-b-2 !border-gray-200">
							<Tab value="insert" size="xs" class="grow">
								<div class="m-1 center-center gap-2">
									<Icon data={faPlus} />
									<span>Insert</span>
								</div>
							</Tab>
							<Tab value="settings" size="xs" class="grow">
								<div class="m-1 center-center gap-2">
									<Icon data={faSliders} />
									<span>Settings</span>
								</div>
							</Tab>
							<Tab value="css" size="xs" class="grow">
								<div class="m-1 center-center gap-2">
									<Icon data={faCode} />
									<span>CSS</span>
								</div>
							</Tab>
							<div slot="content" class="h-full overflow-y-auto pb-4">
								<TabContent class="overflow-auto" value="settings">
									{#if $selectedComponent !== undefined}
										<SettingsPanel />
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
	App editor not available to operators
{/if}
