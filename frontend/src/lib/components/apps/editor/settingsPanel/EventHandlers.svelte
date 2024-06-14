<script lang="ts">
	import type { AppViewerContext, GridItem } from '../../types'
	import { getContext } from 'svelte'
	import EventHandlerItem from './EventHandlerItem.svelte'
	import PanelSection from './common/PanelSection.svelte'

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	export let item: GridItem
	export let ownId: string

	const componentsWithEventHandler = [
		'modalcomponent',
		'drawercomponent',
		'buttoncomponent',
		'formcomponent',
		'formbuttoncomponent',
		'checkboxcomponent',
		'resourceselectcomponent',
		'selectcomponent',
		'tabscomponent',
		'conditionalwrapper',
		'fileinputcomponent',
		's3fileinputcomponent',
		'steppercomponent'
	]
</script>

{#if componentsWithEventHandler.includes(item.data.type)}
	<PanelSection
		title="Event handlers"
		tooltip="Event handlers are used to trigger actions on other components when a specific event occurs. For example, you can trigger a recompute on a component when a button is clicked."
	>
		{#if (`onOpenRecomputeIds` in item.data && Array.isArray(item.data.onOpenRecomputeIds)) || item.data.type === 'modalcomponent' || item.data.type === 'drawercomponent'}
			<EventHandlerItem
				title="on open"
				tooltip="Select components to recompute after this component was opened"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onOpenRecomputeIds}
			/>
		{/if}

		{#if (`onCloseRecomputeIds` in item.data && Array.isArray(item.data.onCloseRecomputeIds)) || item.data.type === 'modalcomponent' || item.data.type === 'drawercomponent'}
			<EventHandlerItem
				title="on close"
				tooltip="Select components to recompute after this component was closed"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onCloseRecomputeIds}
			/>
		{/if}
		{#if (`recomputeIds` in item.data && Array.isArray(item.data.recomputeIds)) || item.data.type === 'buttoncomponent' || item.data.type === 'formcomponent' || item.data.type === 'formbuttoncomponent' || item.data.type === 'checkboxcomponent'}
			<EventHandlerItem
				title="on success"
				tooltip="Select components to recompute after this runnable has successfully run"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.recomputeIds}
			/>
		{/if}
		{#if item.data.type === 'checkboxcomponent'}
			<EventHandlerItem
				title="on toggle"
				tooltip="Contrary to onSuccess, this will only trigger recompute when a human toggle the change, not if it set by a default value or by setValue"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onToggle}
			/>
		{/if}
		{#if item.data.type === 'resourceselectcomponent' || item.data.type === 'selectcomponent'}
			<EventHandlerItem
				title="on select"
				tooltip="Contrary to onSuccess, this will only trigger recompute when a human select an item, not if it set by a default value or by setValue"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onSelect}
			/>
		{/if}
		{#if item.data.type === 'tabscomponent' || item.data.type === 'conditionalwrapper'}
			<EventHandlerItem
				title="on tab change"
				tooltip="Select components to recompute after the selected tab was changed"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onTabChange}
			/>
		{/if}
		{#if item.data.type === 'fileinputcomponent' || item.data.type === 's3fileinputcomponent'}
			<EventHandlerItem
				title="on file change"
				tooltip="Select components to recompute after a file was selected"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onFileChange}
			/>
		{/if}
		{#if item.data.type === 'steppercomponent'}
			<EventHandlerItem
				title="on next"
				tooltip="Select components to recompute after the next button was clicked"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onNext}
			/>
		{/if}
		{#if item.data.type === 'steppercomponent'}
			<EventHandlerItem
				title="on previous"
				tooltip="Select components to recompute after the previous button was clicked"
				items={Object.keys($runnableComponents).filter((id) => id !== ownId)}
				bind:value={item.data.onPrevious}
			/>
		{/if}
	</PanelSection>
{/if}
