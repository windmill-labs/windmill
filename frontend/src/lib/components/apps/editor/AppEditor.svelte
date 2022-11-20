<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { onMount, setContext } from 'svelte'

	import { Pane } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type { App, AppEditorContext, ConnectingInput, EditorMode } from '../types'
	import AppEditorHeader from './AppEditorHeader.svelte'
	import GridEditor from './GridEditor.svelte'

	import type { Schema } from '$lib/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import { Tab } from '$lib/components/common'
	import ComponentList from './componentsPanel/ComponentList.svelte'
	import Icon from 'svelte-awesome'
	import { faPlus, faSliders } from '@fortawesome/free-solid-svg-icons'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import ContextPanel from './contextPanel/ContextPanel.svelte'

	export let app: App
	export let path: string

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>('dnd')
	const schemas = writable<Schema[]>([])
	const resizing = writable<boolean>(false)
	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined
	})

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		app: appStore,
		selectedComponent,
		mode,
		schemas,
		connectingInput,
		resizing
	})

	function clearSelectionOnPreview() {
		if ($mode === 'preview') {
			$selectedComponent = undefined
		}
	}

	let mounted = false
	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs))

	$: $mode && $selectedComponent && clearSelectionOnPreview()
	$: selectedTab = 'settings'

	// If ever the the selected component changes, we need to update the selected tab
	selectedComponent.subscribe(() => {
		if (selectedTab === 'insert') {
			setTimeout(() => {
				selectedTab = 'settings'
			})
		}
	})
</script>

<AppEditorHeader title={app.title} bind:mode={$mode} />
<SplitPanesWrapper>
	{#if $mode !== 'preview'}
		<Pane size={20} minSize={20} maxSize={40}>
			<ContextPanel appPath={path} />
		</Pane>
	{/if}
	<Pane size={60} maxSize={100}>
		<div class="p-4 bg-gray-100 h-full">
			{#if $appStore.grid}
				<GridEditor />
			{/if}
		</div>
	</Pane>
	{#if $mode !== 'preview'}
		<Pane size={20} minSize={20} maxSize={40}>
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
	{/if}
</SplitPanesWrapper>
