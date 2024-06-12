<script lang="ts">
	import type { GridItem } from '../../types'
	import Recompute from './Recompute.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let item: GridItem
	export let ownId: string
</script>

<div>
	<div class="px-3 pt-2 text-sm font-normal flex flex-row items-center gap-1">
		Event handlers
		<Tooltip light>
			Event handlers are used to trigger actions on other components when a specific event occurs.
			For example, you can trigger a recompute on a component when a button is clicked.
		</Tooltip>
	</div>

	{#if (`recomputeIds` in item.data && Array.isArray(item.data.recomputeIds)) || item.data.type === 'buttoncomponent' || item.data.type === 'formcomponent' || item.data.type === 'formbuttoncomponent' || item.data.type === 'checkboxcomponent'}
		<Recompute bind:recomputeIds={item.data.recomputeIds} {ownId} />
	{/if}
	{#if (`onOpenRecomputeIds` in item.data && Array.isArray(item.data.onOpenRecomputeIds)) || item.data.type === 'modalcomponent' || item.data.type === 'drawercomponent'}
		<Recompute
			bind:recomputeIds={item.data.onOpenRecomputeIds}
			{ownId}
			title="Trigger runnables on open"
			tooltip="Select components to recompute after this component was opened"
		/>
	{/if}
	{#if (`onCloseRecomputeIds` in item.data && Array.isArray(item.data.onCloseRecomputeIds)) || item.data.type === 'modalcomponent' || item.data.type === 'drawercomponent'}
		<Recompute
			bind:recomputeIds={item.data.onCloseRecomputeIds}
			{ownId}
			title="Trigger runnables on close"
			tooltip="Select components to recompute after this component was closed"
		/>
	{/if}
	{#if item.data.type === 'checkboxcomponent'}
		<Recompute
			title="Recompute on toggle"
			tooltip={'Contrary to onSuccess, this will only trigger recompute when a human toggle the change, not if it set by a default value or by setValue'}
			documentationLink={undefined}
			bind:recomputeIds={item.data.onToggle}
			{ownId}
		/>
	{/if}
	{#if item.data.type === 'resourceselectcomponent' || item.data.type === 'selectcomponent'}
		<Recompute
			title="Recompute on select"
			tooltip={'Contrary to onSuccess, this will only trigger recompute when a human select an item, not if it set by a default value or by setValue'}
			documentationLink={undefined}
			bind:recomputeIds={item.data.onSelect}
			{ownId}
		/>
	{/if}
</div>
