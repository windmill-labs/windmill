<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { onMount, setContext } from 'svelte'

	import { Pane } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type {
		App,
		AppEditorContext,
		ConnectingInput,
		EditorBreakpoint,
		EditorMode
	} from '../types'
	import AppEditorHeader from './AppEditorHeader.svelte'
	import GridEditor from './GridEditor.svelte'

	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import { Alert, Button, Tab } from '$lib/components/common'
	import ComponentList from './componentsPanel/ComponentList.svelte'
	import Icon from 'svelte-awesome'
	import { faPlus, faSliders } from '@fortawesome/free-solid-svg-icons'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import ContextPanel from './contextPanel/ContextPanel.svelte'
	import { classNames } from '$lib/utils'
	import AppPreview from './AppPreview.svelte'
	import { userStore } from '$lib/stores'

	import InlineScriptsPanel from './inlineScriptsPanel/InlineScriptsPanel.svelte'
	import TablePanel from './TablePanel.svelte'
	import { grid } from 'd3-dag'
	import SettingsPanel from './SettingsPanel.svelte'
	import { fly } from 'svelte/transition'

	export let app: App
	export let path: string
	export let initialMode: EditorMode = 'dnd'

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>(initialMode)
	const breakpoint = writable<EditorBreakpoint>('lg')

	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined
	})

	const runnableComponents = writable<Record<string, () => void>>({})

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		app: appStore,
		lazyGrid: writable([]),
		selectedComponent,
		mode,
		connectingInput,
		breakpoint,
		runnableComponents,
		appPath: path
	})

	let mounted = false

	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs, $worldStore))
	$: previewing = $mode === 'preview'
	$: width = $breakpoint === 'sm' ? 'w-[640px]' : 'min-w-[1080px] w-full'

	let selectedTab: 'insert' | 'settings' = 'insert'
	$: if ($selectedComponent) {
		selectedTab = 'settings'
	} else {
		selectedTab = 'insert'
	}
</script>

{#if !$userStore?.operator}
	{#if initialMode !== 'preview'}
		<AppEditorHeader bind:title={$appStore.title} bind:mode={$mode} bind:breakpoint={$breakpoint} />
	{/if}

	{#if previewing}
		<AppPreview app={$appStore} appPath={path} {breakpoint} />
	{:else}
		<SplitPanesWrapper class="max-w-full overflow-hidden">
			<Pane size={20}>
				<ContextPanel />
			</Pane>
			<Pane size={55}>
				<SplitPanesWrapper horizontal>
					<Pane size={70}>
						<div class="bg-gray-100 w-full p-4 h-full overflow-auto">
							<div class={classNames('bg-gray-100  mx-auto relative min-h-full', width)}>
								{#if $appStore.grid}
									<div class={classNames('w-full p-2 h-full bg-white', width)}>
										<GridEditor />
									</div>
								{/if}
								{#if $connectingInput.opened}
									<div
										class="absolute top-0 left-0 w-full h-full bg-black border-2 bg-opacity-25 z-1 flex justify-center items-center"
									/>
								{/if}
							</div>
						</div>
					</Pane>
					<Pane size={30}>
						<div class="relative h-full w-full">
							<InlineScriptsPanel />
							{#if $connectingInput.opened}
								<div
									class="absolute top-0 left-0 w-full h-full bg-black border-2 bg-opacity-25 z-1 flex justify-center items-center"
								/>
							{/if}</div
						>
					</Pane>
				</SplitPanesWrapper>
			</Pane>
			<Pane size={25} minSize={20} maxSize={33}>
				<div class="relative">
					<Tabs bind:selected={selectedTab}>
						<Tab value="insert" size="xs">
							<div class="m-1 flex flex-row gap-2">
								<Icon data={faPlus} />
								<span>Insert</span>
							</div>
						</Tab>
						<Tab value="settings" size="xs">
							<div class="m-1 flex flex-row gap-2">
								<Icon data={faSliders} />
								<span>Settings</span>
							</div>
						</Tab>
						<svelte:fragment slot="content">
							<TabContent value="settings">
								{#if $selectedComponent !== undefined}
									<SettingsPanel />
								{:else}
									<div class="p-4 text-sm">No component selected.</div>
								{/if}
							</TabContent>
							<TabContent value="insert">
								<ComponentList />
							</TabContent>
						</svelte:fragment>
					</Tabs>
					{#if $connectingInput.opened}
						<div
							class="absolute top-0 left-0 w-full h-full bg-black border-2 bg-opacity-25 z-1 flex justify-center items-center"
						/>
						<div
							class="fixed top-32  p-2 z-10 flex justify-center items-center"
							transition:fly={{ duration: 100, y: -100 }}
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
		</SplitPanesWrapper>
	{/if}
{:else}
	App editor not available to operators
{/if}
