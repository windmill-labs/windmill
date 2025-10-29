<script lang="ts">
	import type { FlowModule, FlowValue, OpenFlow } from '$lib/gen'
	import YAML from 'yaml'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { Alert } from './common'
	import {
		computeFlowModuleDiff,
		splitModuleDiffForViews,
		mergeFlows,
		hasInputSchemaChanged
	} from './flows/flowDiff'
	import { dfs } from './flows/dfs'
	import DiffDrawer from './DiffDrawer.svelte'
	import type { AIModuleAction } from './copilot/chat/flow/core'

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	interface Props {
		beforeYaml: string
		afterYaml: string
	}

	let { beforeYaml, afterYaml }: Props = $props()

	let parseError = $state<string | undefined>(undefined)
	let moduleDiffDrawer: DiffDrawer | undefined = $state(undefined)
	let viewerWidth = $state(SIDE_BY_SIDE_MIN_WIDTH)

	let beforeFlow: OpenFlow | undefined = $derived.by(() => {
		try {
			const parsed = YAML.parse(beforeYaml)
			return parsed as OpenFlow
		} catch (error) {
			parseError = `Error parsing before flow: ${error instanceof Error ? error.message : typeof error === 'string' ? error : 'Unknown error'}`
			return undefined
		}
	})

	let afterFlow: OpenFlow | undefined = $derived.by(() => {
		try {
			const parsed = YAML.parse(afterYaml)
			return parsed as OpenFlow
		} catch (error) {
			parseError = `Error parsing after flow: ${error instanceof Error ? error.message : typeof error === 'string' ? error : 'Unknown error'}`
			return undefined
		}
	})

	// Compute module diff and split for before/after views
	let moduleDiff = $derived(
		beforeFlow && afterFlow ? computeFlowModuleDiff(beforeFlow.value, afterFlow.value) : {}
	)
	let { beforeActions } = $derived(splitModuleDiffForViews(moduleDiff))

	// Detect if input schema has changed
	let inputSchemaModified = $derived(hasInputSchemaChanged(beforeFlow, afterFlow))

	// Determine if we should render side-by-side or unified
	let isSideBySide = $derived(viewerWidth >= SIDE_BY_SIDE_MIN_WIDTH)

	// For unified view, merge both flows to show all modules (added, modified, and removed)
	// In side-by-side view, mark removed modules as 'shadowed' in the After graph
	// In unified view, mark removed modules as 'removed' to show them in red
	let mergedState = $derived.by(() => {
		if (!beforeFlow || !afterFlow) return undefined
		return mergeFlows(beforeFlow.value, afterFlow.value, moduleDiff, isSideBySide)
	})

	// Convert ModuleDiffResult to AIModuleAction map for the merged flow
	// The mergeFlows() function already sets the correct 'before' action ('shadowed' or 'removed')
	let unifiedActions = $derived.by((): Record<string, AIModuleAction> => {
		const actions: Record<string, AIModuleAction> = {}

		for (const [moduleId, diffResult] of Object.entries(mergedState?.diff ?? {})) {
			const action = diffResult.after ?? diffResult.before
			if (action) {
				actions[moduleId] = action
			}
		}

		return actions
	})

	// Helper to find module by ID in a flow
	function getModuleById(flow: FlowValue, moduleId: string): FlowModule | undefined {
		const allModules = dfs(flow.modules ?? [], (m) => m)
		return (
			allModules.find((m) => m?.id === moduleId) ??
			(flow.failure_module?.id === moduleId ? flow.failure_module : undefined) ??
			(flow.preprocessor_module?.id === moduleId ? flow.preprocessor_module : undefined)
		)
	}

	// Callback to show module diff
	function handleShowModuleDiff(moduleId: string) {
		if (!beforeFlow || !afterFlow) return

		// Handle special case for Input schema diff
		if (moduleId === 'Input') {
			moduleDiffDrawer?.openDrawer()
			moduleDiffDrawer?.setDiff({
				mode: 'simple',
				title: 'Flow Input Schema Diff',
				original: { schema: beforeFlow.schema ?? {} },
				current: { schema: afterFlow.schema ?? {} }
			})
			return
		}

		const beforeModule = getModuleById(beforeFlow.value, moduleId)
		const afterModule = getModuleById(afterFlow.value, moduleId)

		if (beforeModule && afterModule) {
			moduleDiffDrawer?.openDrawer()
			moduleDiffDrawer?.setDiff({
				mode: 'simple',
				title: `Module Diff: ${moduleId}`,
				original: beforeModule,
				current: afterModule
			})
		}
	}
</script>

{#if parseError}
	<Alert type="error" title="Parse Error">
		{parseError}
	</Alert>
{:else if beforeFlow && afterFlow}
	<div class="h-full" bind:clientWidth={viewerWidth}>
		{#if isSideBySide}
			<!-- Side-by-side view for wide screens -->
			<div class="grid grid-cols-2 gap-4 h-full">
				<!-- Before (Left) -->
				<div class="flex flex-col h-full border-r border-gray-200 dark:border-gray-700">
					<div
						class="px-4 py-2 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700"
					>
						<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300">Before</h3>
					</div>
					<div class="flex-1 overflow-hidden">
						<FlowGraphV2
							modules={beforeFlow.value.modules}
							failureModule={beforeFlow.value.failure_module}
							preprocessorModule={beforeFlow.value.preprocessor_module}
							earlyStop={beforeFlow.value.skip_expr !== undefined}
							cache={beforeFlow.value.cache_ttl !== undefined}
							moduleActions={beforeActions}
							{inputSchemaModified}
							onShowModuleDiff={handleShowModuleDiff}
							notSelectable={true}
							insertable={false}
							editMode={false}
							download={false}
							scroll={false}
							minHeight={400}
							triggerNode={false}
						/>
					</div>
				</div>

				<!-- After (Right) - Show merged flow with shadowed removed modules -->
				<div class="flex flex-col h-full">
					<div
						class="px-4 py-2 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700"
					>
						<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300">After</h3>
					</div>
					<div class="flex-1 overflow-hidden">
						{#if mergedState}
							<FlowGraphV2
								modules={mergedState.mergedFlow.modules}
								failureModule={mergedState.mergedFlow.failure_module}
								preprocessorModule={mergedState.mergedFlow.preprocessor_module}
								earlyStop={mergedState.mergedFlow.skip_expr !== undefined}
								cache={mergedState.mergedFlow.cache_ttl !== undefined}
								moduleActions={unifiedActions}
								{inputSchemaModified}
								onShowModuleDiff={handleShowModuleDiff}
								notSelectable={true}
								insertable={false}
								editMode={false}
								download={false}
								scroll={false}
								minHeight={400}
								triggerNode={false}
							/>
						{/if}
					</div>
				</div>
			</div>
		{:else}
			<!-- Unified view for narrow screens - show merged flow with all diff colors -->
			{#if mergedState}
				<div class="h-full overflow-hidden">
					<FlowGraphV2
						modules={mergedState.mergedFlow.modules}
						failureModule={mergedState.mergedFlow.failure_module}
						preprocessorModule={mergedState.mergedFlow.preprocessor_module}
						earlyStop={mergedState.mergedFlow.skip_expr !== undefined}
						cache={mergedState.mergedFlow.cache_ttl !== undefined}
						moduleActions={unifiedActions}
						{inputSchemaModified}
						onShowModuleDiff={handleShowModuleDiff}
						notSelectable={true}
						insertable={false}
						editMode={false}
						download={false}
						scroll={false}
						minHeight={400}
						triggerNode={false}
					/>
				</div>
			{/if}
		{/if}
	</div>
	<!-- Nested DiffDrawer for module-level diffs -->
	<DiffDrawer bind:this={moduleDiffDrawer} />
{:else}
	<div class="flex items-center justify-center h-full">
		<p class="text-gray-500">Loading graphs...</p>
	</div>
{/if}
