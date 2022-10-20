<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { faUserGroup } from '@fortawesome/free-solid-svg-icons'

	import { ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from './FlowScriptPicker.svelte'

	export let kind: string
	export let customText: string | undefined = undefined

	type Item = { summary: String; path: String; version?: String }

	let itemPicker: ItemPicker

	const dispatch = createEventDispatcher()

	async function loadItems(): Promise<Item[]> {
		return await ScriptService.listScripts({ workspace: $workspaceStore!, kind })
	}
</script>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, summary) => {
		dispatch('pick', { path, summary })
	}}
	itemName={'Script'}
	extraField="summary"
	{loadItems}
/>

<FlowScriptPicker
	label={customText ?? `Pick a ${kind == 'script' ? '' : kind} script from your workspace`}
	icon={faUserGroup}
	iconColor="text-blue-500"
	on:click={() => itemPicker.openModal()}
/>
