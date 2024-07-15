<script lang="ts">
	import EditableSchemaWrapper from '$lib/components/schema/EditableSchemaWrapper.svelte'

	let schema = {
		type: 'object',
		properties: {
			foo: {
				type: 'string',
				description: '',
				required: [],
				nullable: false,
				default: '',
				disableVariablePicker: false,
				disableCreate: false,
				password: false
			}
		},
		order: ['foo']
	}
</script>

<EditableSchemaWrapper on:change={() => console.log('FOO')} {schema} />
