<script lang="ts">
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'

	import type { FlowValue } from '$lib/gen'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'

	import SchemaViewer from './SchemaViewer.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import FlowModulesViewer from './FlowModulesViewer.svelte'

	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}
	export let initialOpen: number | undefined = undefined

	$: flowFiltered = {
		summary: flow.summary,
		description: flow.description,
		value: flow.value,
		schema: flow.schema
	}

	export let tab: 'ui' | 'json' | 'schema' = 'ui'
	let open: { [id: number]: boolean } = {}
	if (initialOpen) {
		open[initialOpen] = true
	}

	function toAny(x: unknown): any {
		return x as any
	}
</script>

<Tabs bind:selected={tab}>
	<Tab value="ui">Flow rendered</Tab>
	<Tab value="json">Json</Tab>
	<Tab value="schema">Input schema of the flow</Tab>

	<svelte:fragment slot="content">
		<TabContent value="ui">
			<div class="flow-root w-full pb-4">
				<h2 class="my-4">{flow.summary}</h2>
				<SvelteMarkdown source={flow.description ?? ''} />

				<p class="font-black text-lg w-full my-4">
					<span>Inputs</span>
				</p>
				{#if flow.schema && flow.schema.properties && Object.keys(flow.schema.properties).length > 0 && flow.schema}
					<ul class="my-2">
						{#each Object.entries(flow.schema.properties) as [inp, v]}
							<li class="list-disc flex flex-row">
								<FieldHeader
									label={inp}
									required={flow.schema.required?.includes(inp)}
									type={toAny(v)?.type}
									contentEncoding={toAny(v)?.contentEncoding}
									format={toAny(v)?.format}
									itemsType={toAny(v)?.itemsType}
								/><span class="ml-4 mt-2 text-xs"
									>{toAny(v)?.default != undefined
										? 'default: ' + JSON.stringify(toAny(v)?.default)
										: ''}</span
								>
							</li>
						{/each}
					</ul>
				{:else}
					<div class="text-gray-700 text-xs italic mb-4">No inputs</div>
				{/if}
				<FlowModulesViewer
					modules={flow?.value?.modules}
					failureModule={flow?.value?.failure_module}
				/>
			</div>
		</TabContent>
		<TabContent value="json">
			<div class="relative">
				<button
					on:click={async () => {
						await navigator.clipboard.writeText(JSON.stringify(flowFiltered, null, 4))
					}}
					class="absolute default-secondary-button-v2 bg-white/30 right-0 my-2 ml-4"
				>
					copy content</button
				>
				<Highlight language={json} code={JSON.stringify(flowFiltered, null, 4)} />
			</div>
		</TabContent>
		<TabContent value="schema">
			<div class="my-4" />
			<SchemaViewer schema={flow.schema} />
		</TabContent>
	</svelte:fragment>
</Tabs>
