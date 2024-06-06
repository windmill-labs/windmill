<script lang="ts">
	import type { Schema } from '$lib/common'

	import EditableSchemaForm from './EditableSchemaForm.svelte'

	export let schema: Schema | any

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
</script>

<div class="w-full">
	<div>
		<EditableSchemaForm bind:schema uiOnly />
	</div>
</div>
