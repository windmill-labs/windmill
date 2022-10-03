<script lang="ts">
	import type { Schema } from '$lib/common'

	import PageHeader from './PageHeader.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import TabContent from './common/tabs/TabContent.svelte'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import SvelteMarkdown from 'svelte-markdown'

	export let schema: Schema
	export let summary: string
	export let description: string | undefined

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
</script>

<div class="w-full">
	<PageHeader title="UI customisation" />

	<Tabs selected="ui">
		<Tab value="ui">UI</Tab>
		<Tab value="jsonschema">JSON Schema</Tab>
		<svelte:fragment slot="content">
			<TabContent value="ui">
				<div class="grid grid-cols-3 gap-2">
					<div>
						<h2 class="mb-1">Summary</h2>
						<div class="mb-2 md:mb-3 text-sm">
							<textarea bind:value={summary} placeholder="Edit summary" />
							<div class="prose text-sm">
								{summary != '' ? summary : 'No summary'}
							</div>
						</div>
					</div>
					<div class="col-span-2">
						<h2 class="mb-1">Description</h2>
						<div class="mb-2 md:mb-6">
							<div class="prose text-sm">
								<textarea bind:value={description} placeholder="Edit description" />
								<div class="prose text-sm">
									<SvelteMarkdown
										source={description && description != '' ? description : 'No description'}
									/>
								</div>
							</div>
						</div>
					</div>
				</div>
				<div class="bg-blue-100 border-l-4 border-blue-600 text-blue-700 p-4 m-4" role="alert">
					<p class="font-bold">Synchronized with main signature</p>
					<p>
						Argument names, being required or not, and default values are derived from the main
						signature of step 2 and cannot be edited directly. Change the main signature to edit
						them.
					</p>
				</div>
				<SchemaForm {schema} editableSchema={true} />
			</TabContent>
			<TabContent value="jsonschema">
				<Highlight language={json} code={JSON.stringify(schema, null, 4)} />
			</TabContent>
		</svelte:fragment>
	</Tabs>
</div>
