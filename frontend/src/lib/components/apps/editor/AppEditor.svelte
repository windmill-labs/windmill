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

	export let app: App

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
		console.log($staticOutputs, $appStore.grid)
	})

	$: $mode && $selectedComponent && clearSelectionOnPreview()
	$: mounted && ($worldStore = buildWorld($staticOutputs))

	let selectedTab = 'settings'
</script>

<AppEditorHeader title={app.title} bind:mode={$mode} />
<SplitPanesWrapper>
	<Pane>
		<div class="m-8">
			{#if $appStore.grid}
				<GridEditor />
			{/if}
		</div>
	</Pane>
	{#if $mode !== 'preview'}
		<Pane size={30} minSize={30} maxSize={50}>
			<Tabs selected={selectedTab}>
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
