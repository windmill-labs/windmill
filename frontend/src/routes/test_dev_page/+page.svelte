<script lang="ts">
	import EditableSchemaWrapper from '$lib/components/schema/EditableSchemaWrapper.svelte'

	let schema = {
		type: 'object',
		properties: {
			foo: {
				type: 'string'
			}
		}
	}
</script>

<EditableSchemaWrapper on:change={() => console.log('FOO')} {schema} />
