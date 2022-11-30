<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Code } from 'svelte-lucide'

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

<Button
	on:click={() => itemPicker.openDrawer()}
	btnClasses="w-24 truncate"
	size="sm"
	spacingSize="md"
	variant="border"
	color="light"
>
	<div class="flex justify-center flex-col items-center gap-2">
		<Code size="18px" />

		<span class="text-xs">Script</span>
	</div>
</Button>
