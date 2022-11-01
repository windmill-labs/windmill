<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { onMount, setContext } from 'svelte'

	import { Pane } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type { App, AppEditorContext, AppSelection, EditorMode } from '../types'
	import AppEditorHeader from './AppEditorHeader.svelte'
	import SectionsEditor from './SectionsEditor.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'

	export let app: App
	const appStore = writable<App>(app)

	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})

	const selection = writable<AppSelection>(undefined)
	const mode = writable<EditorMode>('width')

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		app: appStore,
		selection,
		mode
	})

	onMount(() => {
		$worldStore = buildWorld($staticOutputs)
	})
</script>

{$mode}
<AppEditorHeader title="Test" bind:mode={$mode} />
<SplitPanesWrapper>
	<Pane minSize={20} maxSize={30} size={20} />
	<Pane>
		<SectionsEditor bind:sections={$appStore.sections} mode={$mode} />
	</Pane>
	<Pane minSize={20} maxSize={30} size={20}>
		<div class="p-4">
			{#if $selection?.sectionIndex !== undefined && $selection?.componentIndex !== undefined}
				<ComponentPanel
					bind:component={$appStore.sections[$selection?.sectionIndex].components[
						$selection?.componentIndex
					]}
				/>
			{/if}
		</div>
	</Pane>
</SplitPanesWrapper>
