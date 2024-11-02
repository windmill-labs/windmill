<script lang="ts">
	import type { Schema } from '$lib/common'

	import EditableSchemaForm from './EditableSchemaForm.svelte'

	export let schema: Schema | any

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
	const yOffset = 80
</script>

<EditableSchemaForm bind:schema uiOnly offset={yOffset} />
