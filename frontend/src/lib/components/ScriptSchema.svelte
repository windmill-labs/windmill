<script lang="ts">
	import type { Schema } from '$lib/common'

	import SchemaForm from './SchemaForm.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import TabContent from './common/tabs/TabContent.svelte'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { Alert } from './common'
	import autosize from 'svelte-autosize'

	export let schema: Schema
	export let description: string | undefined

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
</script>

<div class="w-full">
	<h1 class="my-4">Advanced</h1>

	<Tabs selected="ui">
		<Tab value="ui">Description & Arguments refinement</Tab>
		<Tab value="jsonschema">JSON Schema</Tab>
		<svelte:fragment slot="content">
			<TabContent value="ui">
				<h2 class="border-b pb-1 mt-6 mb-4">Description</h2>
				<textarea
					use:autosize
					bind:value={description}
					placeholder="Edit description"
					class="text-sm"
				/>

				<h2 class="border-b pb-1 mb-4 mt-8">Arguments</h2>
				<Alert type="info" title="Synchronized with main signature">
					Argument names, being required or not, and default values are derived from the main
					signature of step 2 and cannot be edited directly. Change the main signature to edit them.
				</Alert>
				<div class="mt-4" />

				<SchemaForm {schema} editableSchema={true} />
			</TabContent>
			<TabContent value="jsonschema">
				<Highlight language={json} code={JSON.stringify(schema, null, 4)} />
			</TabContent>
		</svelte:fragment>
	</Tabs>
</div>
