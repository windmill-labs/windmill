<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { EditableSchemaFormUi } from '$lib/components/custom_ui'

	import EditableSchemaForm from './EditableSchemaForm.svelte'

	interface Props {
		schema: Schema | any
		customUi?: EditableSchemaFormUi | undefined
	}

	let { schema = $bindable(), customUi = undefined }: Props = $props()

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
</script>

<EditableSchemaForm bind:schema uiOnly {customUi} editTab="inputEditor" />
