<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { onMount, setContext } from 'svelte'

	import { Pane } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type { App, AppEditorContext, AppSelection, ConnectingInput, EditorMode } from '../types'
	import AppEditorHeader from './AppEditorHeader.svelte'
	import SectionsEditor from './SectionsEditor.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'

	import type { Schema } from '$lib/common'
	import SectionPanel from './settingsPanel/SectionPanel.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import { Tab } from '$lib/components/common'
	import ComponentList from './componentsPanel/ComponentList.svelte'

	export let app: App

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})

	const selection = writable<AppSelection | undefined>(undefined)
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
		selection,
		mode,
		schemas,
		connectingInput,
		resizing
	})

	onMount(() => {
		$worldStore = buildWorld($staticOutputs)
	})
</script>

<AppEditorHeader title="Sample app" bind:mode={$mode} />
<SplitPanesWrapper>
	<Pane>
		<SectionsEditor bind:sections={$appStore.sections} mode={$mode} />
	</Pane>
	<Pane minSize={40} maxSize={50} size={30}>
		<Tabs selected="settings">
			<Tab value="settings" size="md">Settings</Tab>
			<Tab value="insert" size="md">Insert</Tab>
			<svelte:fragment slot="content">
				<TabContent value="settings">
					{#if $selection?.sectionIndex !== undefined && $selection?.componentIndex !== undefined}
						<ComponentPanel
							bind:component={$appStore.sections[$selection?.sectionIndex].components[
								$selection?.componentIndex
							]}
							on:remove={() => {
								if (
									$selection?.sectionIndex !== undefined &&
									$selection?.componentIndex !== undefined
								) {
									$appStore.sections[$selection?.sectionIndex].components.splice(
										$selection?.componentIndex,
										1
									)

									$appStore = $appStore
									$selection = undefined
								}
							}}
						/>
					{/if}

					{#if $selection?.sectionIndex !== undefined}
						<SectionPanel
							bind:section={$appStore.sections[$selection.sectionIndex]}
							on:remove={() => {
								if ($selection?.sectionIndex !== undefined) {
									$appStore.sections.splice($selection?.sectionIndex, 1)
									$appStore = $appStore
									$selection = undefined
								}
							}}
						/>
					{/if}
				</TabContent>
				<TabContent value="insert">
					<ComponentList />
				</TabContent>
			</svelte:fragment>
		</Tabs>
	</Pane>
</SplitPanesWrapper>
