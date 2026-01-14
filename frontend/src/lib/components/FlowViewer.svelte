<script lang="ts">
	import type { FlowValue } from '$lib/gen'
	import { Tab, Tabs, TabContent } from './common'
	import SchemaViewer from './SchemaViewer.svelte'
	import FlowGraphViewer from './FlowGraphViewer.svelte'

	import HighlightTheme from './HighlightTheme.svelte'
	import FlowViewerInner from './FlowViewerInner.svelte'
	import FlowInputViewer from './FlowInputViewer.svelte'
	import { Loader2 } from 'lucide-svelte'
	import {
		cleanValueProperties,
		orderedYamlStringify,
		replaceFalseWithUndefined
	} from '$lib/utils'

	type FlowType = {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}

	interface Props {
		flow: FlowType
		/** Flow to compare against (for diff view) */
		compareFlow?: FlowType
		initialOpen?: number | undefined
		noSide?: boolean
		noGraph?: boolean
		tab?: 'ui' | 'raw' | 'schema' | 'diff'
		noSummary?: boolean
		noGraphDownload?: boolean
	}

	let {
		flow,
		compareFlow = undefined,
		initialOpen = undefined,
		noSide = false,
		noGraph = false,
		tab = $bindable(noGraph ? 'schema' : 'ui'),
		noSummary = false,
		noGraphDownload = false
	}: Props = $props()

	function flowToYaml(flowData: FlowType): string {
		const cleaned = structuredClone(
			cleanValueProperties(replaceFalseWithUndefined(flowData))
		)
		return orderedYamlStringify(cleaned)
	}

	const beforeYaml = $derived(compareFlow ? flowToYaml(compareFlow) : '')
	const afterYaml = $derived(flowToYaml(flow))

	let open: { [id: number]: boolean } = {}
	if (initialOpen) {
		open[initialOpen] = true
	}
</script>

<HighlightTheme />

<Tabs bind:selected={tab}>
	{#if !noGraph}
		<Tab value="ui" label="Graph" />
	{/if}
	<Tab value="raw" label="Raw" />
	<Tab value="schema" label="Input Schema" />
	{#if compareFlow}
		<Tab value="diff" label="Diff" />
	{/if}

	{#snippet content()}
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
					<FlowInputViewer schema={flow.schema} />
				{:else}
					<div class="text-secondary text-xs italic mb-4">No inputs</div>
				{/if}

				<FlowGraphViewer download={!noGraphDownload} {noSide} {flow} overflowAuto />
			</div>
		</TabContent>
		<TabContent value="raw">
			<FlowViewerInner {flow} />
		</TabContent>
		<TabContent value="schema">
			<div class="my-4"></div>
			<SchemaViewer schema={flow.schema} />
		</TabContent>
		{#if compareFlow}
			<TabContent value="diff">
				<div class="h-[calc(100vh-300px)] min-h-[400px] mt-4">
					{#await import('$lib/components/FlowYamlGraphDiff.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default {beforeYaml} {afterYaml} />
					{/await}
				</div>
			</TabContent>
		{/if}
	{/snippet}
</Tabs>
