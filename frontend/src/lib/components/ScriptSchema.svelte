<script lang="ts">
	import type { Schema } from '$lib/common'

	import Tab from './common/tabs/Tab.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import TabContent from './common/tabs/TabContent.svelte'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import EditableSchemaForm from './EditableSchemaForm.svelte'

	export let schema: Schema | any

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
</script>

<div class="w-full">
	<Tabs selected="ui">
		<Tab value="ui">Arguments</Tab>
		<Tab value="jsonschema">JSON Schema</Tab>
		<svelte:fragment slot="content">
			<TabContent value="ui">
				<div class="mt-4" />
				<EditableSchemaForm bind:schema />
			</TabContent>
			<TabContent value="jsonschema">
				<Highlight language={json} code={JSON.stringify(schema, null, 4)} />
			</TabContent>
		</svelte:fragment>
	</Tabs>
</div>
