<script lang="ts">
	import type { EditableSchemaWrapperProps } from './editable_schema_wrapper'
	import EditableSchemaWrapper from './EditableSchemaWrapper.svelte'

	let {
		schema: oldSchema,
		...props
	}: EditableSchemaWrapperProps = $props()

	let schema = $state(oldSchema)
</script>

<EditableSchemaWrapper {schema} {...props} />