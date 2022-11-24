<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	export let scripts: Item[]

	type Item = { summary: string }

	let itemPicker: ItemPicker
	const dispatch = createEventDispatcher()

	async function loadItems(): Promise<Item[]> {
		return scripts
	}
</script>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, summary) => {
		dispatch('pick', { path, summary })
	}}
	itemName={'Script'}
	extraField="summary"
	noItemMessage={`There are no inline scripts.<br>
	Click '<span class="font-semibold">Add script</span>' on the left panel to create one.`}
	{loadItems}
/>

<Button size="xs" color="dark" on:click={() => itemPicker.openDrawer()}>
	Pick an inline script
</Button>
