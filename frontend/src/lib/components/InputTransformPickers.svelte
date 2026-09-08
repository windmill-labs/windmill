<script lang="ts">
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { watch } from 'runed'
	import { Plus } from 'lucide-svelte'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'

	interface Props {
		/** The transforms being edited. A picked variable is written into `pickForField`'s. */
		args?: Record<string, any>
		/** Which field the open picker is picking for, set by whichever row opened it. */
		pickForField?: string | undefined
		workspace?: string | undefined
		/** Handed back so every field's form can open them. */
		itemPicker?: ItemPicker | undefined
		variableEditor?: VariableEditor | undefined
	}

	let {
		args = {},
		pickForField = undefined,
		workspace = undefined,
		itemPicker = $bindable(),
		variableEditor = $bindable()
	}: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	watch(
		() => ws,
		() => itemPicker?.reloadItems()
	)
</script>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, _) => {
		if (pickForField) {
			args[pickForField].value = '$var:' + path
		}
	}}
	itemName="Variable"
	extraField="path"
	loadItems={async () =>
		(await VariableService.listVariable({ workspace: ws ?? '' })).map((x) => ({
			name: x.path,
			...x
		}))}
>
	{#snippet submission()}
		<div class="flex flex-row-reverse w-full border-t border-gray-200 rounded-bl-lg rounded-br-lg">
			<Button
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: Plus }}
				on:click={() => {
					variableEditor?.initNew?.()
				}}
			>
				New variable
			</Button>
		</div>
	{/snippet}
</ItemPicker>

<VariableEditor bind:this={variableEditor} workspace={ws} />
