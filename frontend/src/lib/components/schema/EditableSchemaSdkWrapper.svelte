<script lang="ts">
	import { readFieldsRecursively } from '$lib/utils'
	import type { EditableSchemaWrapperProps } from './editable_schema_wrapper'
	import EditableSchemaWrapper from './EditableSchemaWrapper.svelte'

	let { schema: oldSchema, ...props }: EditableSchemaWrapperProps = $props()

	let schema = $state(oldSchema)

	$effect(() => {
		console.log('schema 2', schema)
	})

	$effect(() => {
		readFieldsRecursively(schema)
		console.log('schema 3', schema)
	})
</script>

{JSON.stringify(schema)}

<EditableSchemaWrapper bind:schema {...props} />
