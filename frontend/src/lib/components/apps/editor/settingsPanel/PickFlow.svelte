<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { faUserGroup } from '@fortawesome/free-solid-svg-icons'
	import { FlowService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'

	type Item = { summary: String; path: String; version?: String }

	let itemPicker: ItemPicker
	const dispatch = createEventDispatcher()

	async function loadItems(): Promise<Item[]> {
		return await FlowService.listFlows({ workspace: $workspaceStore! })
	}
</script>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, summary) => {
		dispatch('pick', { path, summary })
	}}
	itemName="Flow"
	extraField="summary"
	{loadItems}
/>

<FlowScriptPicker
	label={`Pick flow from workspace`}
	icon={faUserGroup}
	iconColor="text-blue-500"
	on:click={() => itemPicker.openDrawer()}
/>
