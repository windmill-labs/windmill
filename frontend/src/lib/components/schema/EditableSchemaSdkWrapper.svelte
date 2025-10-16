<script lang="ts">
	import { readFieldsRecursively } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import type { EditableSchemaWrapperProps } from './editable_schema_wrapper'
	import EditableSchemaWrapper from './EditableSchemaWrapper.svelte'

	let {
		schema: oldSchema,
		onSchemaChange,
		...props
	}: EditableSchemaWrapperProps & { onSchemaChange?: (schema: any) => void } = $props()

	let schema = $state(oldSchema)

	let lastSchema = $state<any>(undefined)
	$effect(() => {
		if (onSchemaChange && schema) {
			readFieldsRecursively(schema)
			let newSchema = $state.snapshot(schema)
			if (!deepEqual(lastSchema, newSchema)) {
				lastSchema = newSchema
				onSchemaChange(newSchema)
			}
		}
	})
</script>

<EditableSchemaWrapper bind:schema {...props} />
