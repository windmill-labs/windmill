<script lang="ts">
	import type ItemPicker from './ItemPicker.svelte'
	import type VariableEditor from './VariableEditor.svelte'
	import VariablePicker from './VariablePicker.svelte'

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
</script>

<VariablePicker
	bind:itemPicker
	bind:variableEditor
	{workspace}
	onPick={(path) => {
		if (pickForField) {
			args[pickForField].value = '$var:' + path
		}
	}}
/>
