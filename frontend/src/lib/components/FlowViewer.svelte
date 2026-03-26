<script lang="ts">
	import { untrack } from 'svelte'
	import { type FlowValue, FlowService } from '$lib/gen'
	import { Tab, Tabs, TabContent } from './common'
	import SchemaViewer from './SchemaViewer.svelte'
	import FlowGraphViewer from './FlowGraphViewer.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { orderedYamlStringify, cleanValueProperties, replaceFalseWithUndefined } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import { watch } from 'runed'

	import HighlightTheme from './HighlightTheme.svelte'
	import FlowViewerInner from './FlowViewerInner.svelte'
	import FlowInputViewer from './FlowInputViewer.svelte'

	interface PreviousFlow {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}

	export type TabValue = 'ui' | 'raw' | 'schema' | 'diff'

	interface Props {
		flow: {
			summary: string
			description?: string
			value: FlowValue
			schema?: any
		}
		initialOpen?: number | undefined
		noSide?: boolean
		noGraph?: boolean
		initTab?: TabValue
		selectedTab?: TabValue
		hideTabs?: boolean
		noSummary?: boolean
		noInput?: boolean
		hideDefaultInputs?: boolean
		showStepHint?: boolean
		noGraphDownload?: boolean
		availableVersions?: Array<{ id: number; deployment_msg?: string }>
		selectedVersionId?: number
		graphContent?: import('svelte').Snippet
	}

	let {
		flow,
		initialOpen = undefined,
		noSide = false,
		noGraph = false,
		availableVersions = undefined,
		initTab = undefined,
		selectedTab = $bindable(),
		hideTabs = false,
		noSummary = false,
		noInput = false,
		hideDefaultInputs = false,
		showStepHint = false,
		noGraphDownload = false,
		selectedVersionId = undefined,
		graphContent = undefined
	}: Props = $props()

	let open: { [id: number]: boolean } = {}
	const untrackedInitialOpen = untrack(() => initialOpen)
	if (untrackedInitialOpen) {
		open[untrackedInitialOpen] = true
	}

	let previousVersionId: number | undefined = $state(undefined)
	let previousFlow: PreviousFlow | undefined = $state(undefined)
	let tab: TabValue = $state(untrack(() => selectedTab ?? initTab) ?? 'diff')

	// Sync bindable selectedTab with internal tab state
	$effect(() => {
		selectedTab = tab
	})
	$effect(() => {
		if (selectedTab !== undefined && selectedTab !== tab) {
			tab = selectedTab
		}
	})

	let previousFlowCache: Record<number, PreviousFlow> = {}

	async function loadPreviousFlow(version: number) {
		try {
			if (previousFlowCache[version]) {
				previousFlow = previousFlowCache[version]
				return
			}
			previousFlow = await FlowService.getFlowVersion({
				workspace: $workspaceStore!,
				version
			})
			previousFlowCache[version] = previousFlow
		} catch (e) {
			console.error(e)
			previousFlow = undefined
		}
	}

	// Load previous flow when previousVersionId changes
	$effect.pre(() => {
		if (previousVersionId !== undefined) {
			loadPreviousFlow(previousVersionId)
		} else {
			previousFlow = undefined
		}
	})

	$effect.pre(() => {
		if (initTab) {
			return
		}
		if (availableVersions && availableVersions.length > 0) {
			tab = 'diff'
		} else {
			if (noGraph) {
				tab = 'schema'
			} else {
				tab = 'ui'
			}
		}
	})

	// Auto-select first available version and validate current selection
	watch([() => availableVersions, () => selectedVersionId], () => {
		if (availableVersions && availableVersions.length > 0) {
			previousVersionId = availableVersions[0].id
		} else {
			previousVersionId = undefined
		}
	})

	let currentFlowYaml = $derived.by(() => {
		const metadata = structuredClone(cleanValueProperties(replaceFalseWithUndefined(flow)))
		return orderedYamlStringify(metadata)
	})

	let previousFlowYaml = $derived.by(() => {
		if (!previousFlow) return undefined
		const metadata = structuredClone(cleanValueProperties(replaceFalseWithUndefined(previousFlow)))
		return orderedYamlStringify(metadata)
	})
</script>

<HighlightTheme />

<Tabs bind:selected={tab} {hideTabs}>
	{#if availableVersions && availableVersions.length > 0}
		<Tab value="diff" label="Diff" />
	{/if}
	{#if !noGraph}
		<Tab value="ui" label="Graph" />
	{/if}
	<Tab value="raw" label="Raw" />
	<Tab value="schema" label="Input Schema" />

	{#snippet content()}
		{#if availableVersions && availableVersions.length > 0}
			<TabContent value="diff">
				<div class="flex flex-col gap-2 h-full">
					<div class="flex flex-row items-center gap-2 py-2">
						<div class="text-xs">Compare with:</div>
						<select bind:value={previousVersionId} class="!text-xs !w-40">
							{#each availableVersions as version (version.id)}
								<option value={version.id} class="!text-xs">
									{version.deployment_msg ?? `Version ${version.id}`}
								</option>
							{/each}
						</select>
					</div>

					{#if previousFlowYaml}
						<div class="h-[calc(100vh-150px)] min-h-[400px]">
							{#await import('$lib/components/FlowDiffViewer.svelte')}
								<Loader2 class="animate-spin" />
							{:then Module}
								<Module.default beforeYaml={previousFlowYaml} afterYaml={currentFlowYaml} />
							{/await}
						</div>
					{:else}
						<Loader2 class="animate-spin" />
					{/if}
				</div>
			</TabContent>
		{/if}
		<TabContent value="ui">
			{#if graphContent}
				{@render graphContent()}
			{:else}
				<div class="flow-root w-full pb-4">
					{#if showStepHint}
						<p class="text-2xs text-tertiary py-1">Click on a step to see its details</p>
					{/if}
					{#if !noSummary}
						<h2 class="my-4">{flow.summary}</h2>
						<div>{flow.description ?? ''}</div>
					{/if}

					{#if !noInput}
						<p class="font-black text-lg w-full my-4">
							<span>Flow Input</span>
						</p>
						{#if flow.schema && flow.schema.properties && Object.keys(flow.schema.properties).length > 0 && flow.schema}
							<FlowInputViewer schema={flow.schema} />
						{:else}
							<div class="text-secondary text-xs italic mb-4">No inputs</div>
						{/if}
					{/if}

					<FlowGraphViewer
						download={!noGraphDownload}
						{noSide}
						{hideDefaultInputs}
						{flow}
						overflowAuto
					/>
				</div>
			{/if}
		</TabContent>
		<TabContent value="raw">
			<FlowViewerInner {flow} />
		</TabContent>
		<TabContent value="schema">
			<div class="my-4"></div>
			<SchemaViewer schema={flow.schema} />
		</TabContent>
	{/snippet}
</Tabs>
