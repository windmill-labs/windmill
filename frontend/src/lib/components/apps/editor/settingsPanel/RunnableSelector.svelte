<script lang="ts">
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { faMousePointer } from '@fortawesome/free-solid-svg-icons'

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

<Button
	on:click={() => itemPicker?.openDrawer()}
	size="sm"
	spacingSize="md"
	color="blue"
	startIcon={{ icon: faMousePointer }}
>
	Pick
</Button>
