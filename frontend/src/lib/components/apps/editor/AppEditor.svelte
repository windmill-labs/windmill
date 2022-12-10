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
	import { Tab } from '$lib/components/common'
	import ComponentList from './componentsPanel/ComponentList.svelte'
	import Icon from 'svelte-awesome'
	import { faPlus, faSliders } from '@fortawesome/free-solid-svg-icons'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import ContextPanel from './contextPanel/ContextPanel.svelte'
	import { classNames } from '$lib/utils'
	import AppPreview from './AppPreview.svelte'

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
		selectedComponent,
		mode,
		connectingInput,
		breakpoint,
		runnableComponents
	})

	let mounted = false
	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs))
	$: selectedTab = $selectedComponent ? 'settings' : 'insert'
	$: previewing = $mode === 'preview'

	$: width = $breakpoint === 'sm' ? 'w-[640px]' : 'w-full '
</script>

{#if initialMode !== 'preview'}
	<AppEditorHeader bind:title={$appStore.title} bind:mode={$mode} bind:breakpoint={$breakpoint} />
{/if}

{#if previewing}
	<AppPreview app={$appStore} />
{:else}
	<SplitPanesWrapper class="max-w-full overflow-hidden">
		<Pane size={20} minSize={15} maxSize={40}>
			<ContextPanel appPath={path} />
		</Pane>
		<Pane size={60}>
			<div class="p-4 bg-gray-100 min-h-full w-full relative">
				{#if $appStore.grid}
					<div class={classNames('mx-auto h-full', width)}>
						<GridEditor />
					</div>
				{/if}
				{#if $connectingInput.opened}
					<div
						class="absolute top-0 left-0 w-full h-full bg-black border-2 bg-opacity-25 z-1 flex justify-center items-center"
					/>
				{/if}
			</div>
		</Pane>
		<Pane size={25} minSize={25} maxSize={40}>
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
							{#each $appStore.grid as gridItem (gridItem.id)}
								{#if gridItem.data.id === $selectedComponent}
									<ComponentPanel bind:component={gridItem.data} />
								{/if}
							{/each}
						{/if}
						{#if $selectedComponent === undefined}
							<div class="p-4 text-sm">No component selected.</div>
						{/if}
					</TabContent>
					<TabContent value="insert">
						<ComponentList />
					</TabContent>
				</svelte:fragment>
			</Tabs>
		</Pane>
	</SplitPanesWrapper>
{/if}
