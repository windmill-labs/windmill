<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { FlowService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Wind } from 'lucide-svelte'

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

<Button
	on:click={() => itemPicker.openDrawer()}
	btnClasses="w-24 truncate"
	size="sm"
	spacingSize="md"
	variant="border"
	color="light"
>
	<div class="flex justify-center flex-col items-center gap-2">
		<Wind size="18px" />

		<span class="text-xs">Flow</span>
	</div>
</Button>
