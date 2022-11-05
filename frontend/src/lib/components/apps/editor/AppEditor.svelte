<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { faBarChart, faDisplay, faPieChart, faTable } from '@fortawesome/free-solid-svg-icons'
	import { faWpforms } from '@fortawesome/free-brands-svg-icons'

	import { onMount, setContext } from 'svelte'
	import Icon from 'svelte-awesome'

	import { Pane } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type { App, AppEditorContext, AppSelection, EditorMode, AppInputTransform } from '../types'
	import AppEditorHeader from './AppEditorHeader.svelte'
	import SectionsEditor from './SectionsEditor.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import { dndzone } from 'svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import type { Schema } from '$lib/common'
	import SectionPanel from './settingsPanel/SectionPanel.svelte'

	const flipDurationMs = 200

	export let app: App
	const appStore = writable<App>(app)

	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})

	const selection = writable<AppSelection | undefined>(undefined)
	const mode = writable<EditorMode>('dnd')
	const schemas = writable<Schema[]>([])

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		app: appStore,
		selection,
		mode,
		schemas
	})

	onMount(() => {
		$worldStore = buildWorld($staticOutputs)
	})

	let c = [
		{
			id: 'displaycomponent',
			name: 'Display component',
			icon: faDisplay
		},
		{
			id: 'runformcomponent',
			name: 'Run form',
			icon: faWpforms
		},
		{
			id: 'piechart',
			name: 'Pie chart',
			icon: faPieChart
		},
		{
			id: 'barchart',
			name: 'Bar chart',
			icon: faBarChart
		},
		{
			id: 'table',
			name: 'Table',
			icon: faTable
		}
	]
</script>

<AppEditorHeader title="Sample app" bind:mode={$mode} />
<SplitPanesWrapper>
	<Pane minSize={20} maxSize={30} size={20}>
		<div class="bg-gray-100 h-full">
			<div
				class="grid grid-cols-2 gap-2 p-2 "
				use:dndzone={{
					items: c,
					flipDurationMs,
					type: 'component',
					dropTargetStyle: {
						outline: 'dashed white',
						outlineOffset: '2px'
					},
					dragDisabled: false,
					dropFromOthersDisabled: true
				}}
				on:consider={(e) => {
					c = c
				}}
				on:finalize={(e) => {
					c = e.detail.items
				}}
			>
				{#each c as component (component.id)}
					<div
						class="border shadow-sm h-24 p-2 flex flex-col gap-2 items-center justify-center bg-white rounded-md"
						animate:flip={{ duration: flipDurationMs }}
					>
						<Icon data={component.icon} scale={1.6} />
						<div class="text-xs">{component.name}</div>
					</div>
				{/each}
			</div></div
		>
	</Pane>
	<Pane>
		<SectionsEditor bind:sections={$appStore.sections} mode={$mode} />
	</Pane>
	<Pane minSize={20} maxSize={30} size={20}>
		{#if $selection?.sectionIndex !== undefined && $selection?.componentIndex !== undefined}
			<ComponentPanel
				bind:component={$appStore.sections[$selection?.sectionIndex].components[
					$selection?.componentIndex
				]}
				on:remove={() => {
					if ($selection?.sectionIndex !== undefined && $selection?.componentIndex !== undefined) {
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
	</Pane>
</SplitPanesWrapper>
