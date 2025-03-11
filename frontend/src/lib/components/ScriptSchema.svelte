<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { EditableSchemaFormUi } from '$lib/components/custom_ui'

	import EditableSchemaForm from './EditableSchemaForm.svelte'

	export let schema: Schema | any
	export let customUi: EditableSchemaFormUi | undefined = undefined

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
</script>

<EditableSchemaForm bind:schema uiOnly {customUi} editTab="inputEditor" />
