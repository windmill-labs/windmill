<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	export let kind: string

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
		dispatch('pick', { path, summary, kind })
	}}
	itemName={'Script'}
	extraField="summary"
	{loadItems}
/>

<Button size="xs" color="dark" on:click={() => itemPicker.openDrawer()}>
	Pick script from workspace
</Button>
