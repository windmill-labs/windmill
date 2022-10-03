<script lang="ts">
	import { faUserGroup } from '@fortawesome/free-solid-svg-icons'

	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { hubScripts } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { Script } from '$lib/gen'
	import type { HubItem } from './model'

	export let kind: Script.kind

	let items: HubItem[]
	$: items = $hubScripts?.filter((x) => x.kind == kind) ?? []
	let itemPicker: ItemPicker

	const dispatch = createEventDispatcher()
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
	label={`Pick a ${kind == Script.kind.SCRIPT ? '' : kind} script from the Hub`}
	icon={faUserGroup}
	iconColor="text-blue-500"
	on:click={() => itemPicker.openModal()}
/>
