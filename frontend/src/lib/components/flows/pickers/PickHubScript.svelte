<script lang="ts">
	import { faUserGroup } from '@fortawesome/free-solid-svg-icons'

	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import { hubScripts } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { Script } from '$lib/gen'
	import type { HubItem } from './model'

	export let kind: Script.kind
	export let customText: string | undefined = undefined

	let items: HubItem[]
	$: items = $hubScripts?.filter((x) => x.kind == kind) ?? []
	let itemPicker: ItemPicker

	const dispatch = createEventDispatcher()
</script>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, summary) => {
		dispatch('pick', { path, summary, kind })
	}}
	itemName={'Script'}
	extraField="summary"
	loadItems={async () => {
		return items
	}}
	noItemMessage="Hub not reachable. If your environment is air gapped, contact sales@windmill.dev to setup a local mirror."
/>

<FlowScriptPicker
	label={customText ?? `${kind == Script.kind.SCRIPT ? 'Script' : `${kind} script`} from the Hub`}
	icon={faUserGroup}
	iconColor="text-blue-500"
	on:click={() => itemPicker.openModal()}
/>
