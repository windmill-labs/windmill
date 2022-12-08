<script lang="ts">
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { faClipboard } from '@fortawesome/free-solid-svg-icons'
	import type { FlowValue } from '$lib/gen'
	import { Tab, Tabs, TabContent, Button } from './common'
	import SchemaViewer from './SchemaViewer.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { copyToClipboard } from '../utils'
	import FlowGraphViewer from './FlowGraphViewer.svelte'

	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}
	export let initialOpen: number | undefined = undefined
	export let noSide = false

	$: flowFiltered = {
		summary: flow.summary,
		description: flow.description,
		value: flow.value,
		schema: flow.schema
	}

	export let tab: 'ui' | 'json' | 'schema' = 'ui'
	export let noSummary = false

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
				{#if !noSummary}
					<h2 class="my-4">{flow.summary}</h2>
					<SvelteMarkdown source={flow.description ?? ''} />
				{/if}

				<p class="font-black text-lg w-full my-4">
					<span>Flow Input</span>
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
				<FlowGraphViewer {noSide} {flow} overflowAuto />
			</div>
		</TabContent>
		<TabContent value="json">
			<div class="relative">
				<Button
					on:click={() => copyToClipboard(JSON.stringify(flowFiltered, null, 4))}
					color="dark"
					variant="border"
					size="sm"
					startIcon={{ icon: faClipboard }}
					btnClasses="absolute top-2 right-2"
				>
					Copy content
				</Button>
				<Highlight language={json} code={JSON.stringify(flowFiltered, null, 4)} />
			</div>
		</TabContent>
		<TabContent value="schema">
			<div class="my-4" />
			<SchemaViewer schema={flow.schema} />
		</TabContent>
	</svelte:fragment>
</Tabs>
