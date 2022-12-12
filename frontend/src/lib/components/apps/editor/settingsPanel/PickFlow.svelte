<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { FlowService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Icon } from 'svelte-awesome'
	import { faBarsStaggered } from '@fortawesome/free-solid-svg-icons'

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
		<Icon data={faBarsStaggered} scale={0.8} class="mr-1" />

		<span class="text-xs">Flow</span>
	</div>
</Button>
