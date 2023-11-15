<script lang="ts">
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import type { FlowValue } from '$lib/gen'
	import { Tab, Tabs, TabContent, Button } from './common'
	import SchemaViewer from './SchemaViewer.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import { copyToClipboard } from '../utils'
	import FlowGraphViewer from './FlowGraphViewer.svelte'
	import { setContext } from 'svelte'
	import {
		SVELVET_CONTEXT_KEY,
		type SvelvetSettingsContext
	} from './graph/svelvet/container/models'
	import { Clipboard } from 'lucide-svelte'
	import YAML from 'yaml'
	import { yaml } from 'svelte-highlight/languages'

	setContext<SvelvetSettingsContext>(SVELVET_CONTEXT_KEY, {
		fullHeight: true
	})

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

	export let noGraph: boolean = false

	export let tab: 'ui' | 'raw' | 'schema' = noGraph ? 'schema' : 'ui'
	export let noSummary = false

	let rawType: 'json' | 'yaml' = 'yaml'

	let open: { [id: number]: boolean } = {}
	if (initialOpen) {
		open[initialOpen] = true
	}

	function toAny(x: unknown): any {
		return x as any
	}
</script>

<Tabs bind:selected={tab}>
	{#if !noGraph}
		<Tab value="ui">Graph</Tab>
	{/if}
	<Tab value="raw">Raw</Tab>
	<Tab value="schema">Input Schema</Tab>

	<svelte:fragment slot="content">
		<TabContent value="ui">
			<div class="flow-root w-full pb-4">
				{#if !noSummary}
					<h2 class="my-4">{flow.summary}</h2>
					<div>{flow.description ?? ''}</div>
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
								/><span class="ml-4 mt-2 text-xs"
									>{toAny(v)?.default != undefined
										? 'default: ' + JSON.stringify(toAny(v)?.default)
										: ''}</span
								>
							</li>
						{/each}
					</ul>
				{:else}
					<div class="text-secondary text-xs italic mb-4">No inputs</div>
				{/if}
				<FlowGraphViewer download {noSide} {flow} overflowAuto />
			</div>
		</TabContent>
		<TabContent value="raw"
			><Tabs bind:selected={rawType} wrapperClass="mt-4">
				<Tab value="yaml">YAML</Tab>
				<Tab value="json">JSON</Tab>
				<svelte:fragment slot="content">
					<div class="relative pt-2">
						<Button
							on:click={() =>
								copyToClipboard(
									rawType === 'yaml'
										? YAML.stringify(flowFiltered)
										: JSON.stringify(flowFiltered, null, 4)
								)}
							color="light"
							variant="border"
							size="xs"
							startIcon={{ icon: Clipboard }}
							btnClasses="absolute top-2 right-2 w-min"
						>
							Copy content
						</Button>
						<Highlight
							class="overflow-auto px-1"
							language={rawType === 'yaml' ? yaml : json}
							code={rawType === 'yaml'
								? YAML.stringify(flowFiltered)
								: JSON.stringify(flowFiltered, null, 4)}
						/>
					</div>
				</svelte:fragment>
			</Tabs>
		</TabContent>
		<TabContent value="schema">
			<div class="my-4" />
			<SchemaViewer schema={flow.schema} />
		</TabContent>
	</svelte:fragment>
</Tabs>
