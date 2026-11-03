<script lang="ts">
	import { VariableService } from '$lib/gen'
	import { watch } from 'runed'
	import { Plus } from 'lucide-svelte'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		onPick: (path: string, name: string) => void
		workspace?: string | undefined
		tooltip?: string
		/** Adds an Edit/View button to each row. */
		editable?: boolean
		/** Handed back for forms whose fields open the picker or a linked variable themselves. */
		itemPicker?: ItemPicker | undefined
		variableEditor?: VariableEditor | undefined
	}

	let {
		onPick,
		workspace = undefined,
		tooltip = 'Variables are dynamic values that have a key associated to them and can be retrieved during the execution of a Script or Flow.',
		editable = false,
		itemPicker = $bindable(),
		variableEditor = $bindable()
	}: Props = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)

	watch(
		() => ws,
		() => itemPicker?.reloadItems()
	)

	export function openDrawer() {
		itemPicker?.openDrawer()
	}
</script>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={onPick}
	itemName="Variable"
	{tooltip}
	documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
	extraField="path"
	loadItems={async () =>
		(await VariableService.listVariable({ workspace: ws ?? '' })).map((x) => ({
			name: x.path,
			...x
		}))}
	buttons={editable ? { 'Edit/View': (path) => variableEditor?.editVariable(path) } : {}}
>
	{#snippet submission()}
		<Button
			variant="default"
			unifiedSize="md"
			startIcon={{ icon: Plus }}
			onClick={() => variableEditor?.initNew()}
		>
			New variable
		</Button>
	{/snippet}
</ItemPicker>

<!-- Creating one from the picker must refresh the list it is picking from. -->
<VariableEditor
	bind:this={variableEditor}
	workspace={ws}
	onSaved={() => itemPicker?.reloadItems()}
/>
