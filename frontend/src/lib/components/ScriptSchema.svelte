<script lang="ts">
	import type { Schema } from '$lib/common'

	import SchemaForm from './SchemaForm.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import TabContent from './common/tabs/TabContent.svelte'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import SvelteMarkdown from 'svelte-markdown'
	import { Alert } from './common'

	export let schema: Schema
	export let summary: string
	export let description: string | undefined

	export function setSchema(newSchema: Schema) {
		schema = newSchema
	}
</script>

<div class="w-full">
	<h1 class="my-4">Advanced</h1>

	<Tabs selected="ui">
		<Tab value="ui">UI</Tab>
		<Tab value="jsonschema">JSON Schema</Tab>
		<svelte:fragment slot="content">
			<TabContent value="ui">
				<h2 class="border-b pb-1 mt-6">Display</h2>
				<div class="grid grid-cols-3 gap-2 mt-4">
					<label>
						<div class="text-gray-700 text-sm">Summary</div>
						<textarea bind:value={summary} placeholder="Edit summary" class="text-sm" />
					</label>
					<label class="col-span-2">
						<div class="text-gray-700 text-sm">Description</div>
						<textarea
							bind:value={description}
							placeholder="Edit description. Markdown accepted."
							class="text-sm"
						/>
						<div class="mt-1 px-2">
							{#if description}
								<div
									class="prose !max-w-full !max-h-48 text-xs shadow-inner shadow-blue overflow-auto mt-1"
								>
									<SvelteMarkdown source={description} />
								</div>
							{:else}
								<div class="text-xs text-gray-500"> Enter a description to see the preview </div>
							{/if}
						</div>
					</label>
				</div>
				<h2 class="border-b pb-1 my-4">Arguments</h2>
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
