<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { faUserGroup } from '@fortawesome/free-solid-svg-icons'

	import { ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from './FlowScriptPicker.svelte'

	export let isTrigger: boolean

	type Item = { summary: String; path: String; version?: String }

	let items: Item[] = []
	let itemPicker: ItemPicker

	const dispatch = createEventDispatcher()

	async function loadItems(): Promise<void> {
		items = await ScriptService.listScripts({ workspace: $workspaceStore! })
	}

	$: {
		if ($workspaceStore) {
			loadItems()
		}
	}
</script>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path) => {
		dispatch('pick', { path })
	}}
	itemName={'Script'}
	extraField="summary"
	loadItems={async () => {
		return items
	}}
/>

<FlowScriptPicker
	label={`Pick a ${isTrigger ? 'trigger ' : ''}script from your workspace`}
	icon={faUserGroup}
	iconColor="text-blue-500"
	on:click={() => itemPicker.openModal()}
/>
