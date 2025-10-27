<script lang="ts">
	import type { FlowModule, FlowValue } from '$lib/gen'
	import YAML from 'yaml'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { Alert } from './common'
	import { computeFlowModuleDiff, splitModuleDiffForViews } from './flows/flowDiff'
	import { mergeFlows } from './flows/flowMerge'
	import { dfs } from './flows/dfs'
	import DiffDrawer from './DiffDrawer.svelte'
	import type { AIModuleAction } from './copilot/chat/flow/core'

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	interface Props {
		beforeYaml: string
		afterYaml: string
	}

	let { beforeYaml, afterYaml }: Props = $props()

	let beforeFlow = $state<FlowValue | undefined>(undefined)
	let afterFlow = $state<FlowValue | undefined>(undefined)
	let parseError = $state<string | undefined>(undefined)
	let moduleDiffDrawer: DiffDrawer | undefined = $state(undefined)
	let viewerWidth = $state(SIDE_BY_SIDE_MIN_WIDTH)

	// Parse YAML into FlowValue objects
	$effect(() => {
		try {
			parseError = undefined
			beforeFlow = YAML.parse(beforeYaml).value as FlowValue
			afterFlow = YAML.parse(afterYaml).value as FlowValue
		} catch (error) {
			parseError = error instanceof Error ? error.message : 'Failed to parse YAML'
			beforeFlow = undefined
			afterFlow = undefined
		}
	})

	// Compute module diff and split for before/after views
	let moduleDiff = $derived(
		beforeFlow && afterFlow ? computeFlowModuleDiff(beforeFlow, afterFlow) : {}
	)
	let { beforeActions, afterActions } = $derived(splitModuleDiffForViews(moduleDiff))

	// Determine if we should render side-by-side or unified
	let isSideBySide = $derived(viewerWidth >= SIDE_BY_SIDE_MIN_WIDTH)

	// For unified view, merge both flows to show all modules (added, modified, and removed)
	let mergedFlow = $derived(
		beforeFlow && afterFlow ? mergeFlows(beforeFlow, afterFlow, moduleDiff) : undefined
	)

	// For unified view, combine all actions to show on the merged flow
	let unifiedActions = $derived.by((): Record<string, AIModuleAction> => {
		const unified: Record<string, AIModuleAction> = {}
		// Include all actions from the diff
		for (const [moduleId, diffResult] of Object.entries(moduleDiff)) {
			if (diffResult.after) {
				// Module exists in after (added or modified)
				unified[moduleId] = diffResult.after
			} else if (diffResult.before === 'removed') {
				// Module was removed - now will appear in merged flow with red color
				unified[moduleId] = 'removed'
			}
		}
		return unified
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

		const beforeModule = getModuleById(beforeFlow, moduleId)
		const afterModule = getModuleById(afterFlow, moduleId)

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
							modules={beforeFlow.modules}
							failureModule={beforeFlow.failure_module}
							preprocessorModule={beforeFlow.preprocessor_module}
							earlyStop={beforeFlow.skip_expr !== undefined}
							cache={beforeFlow.cache_ttl !== undefined}
							moduleActions={beforeActions}
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

				<!-- After (Right) -->
				<div class="flex flex-col h-full">
					<div
						class="px-4 py-2 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700"
					>
						<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300">After</h3>
					</div>
					<div class="flex-1 overflow-hidden">
						<FlowGraphV2
							modules={afterFlow.modules}
							failureModule={afterFlow.failure_module}
							preprocessorModule={afterFlow.preprocessor_module}
							earlyStop={afterFlow.skip_expr !== undefined}
							cache={afterFlow.cache_ttl !== undefined}
							moduleActions={afterActions}
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
			</div>
		{:else}
			<!-- Unified view for narrow screens - show merged flow with all diff colors -->
			{#if mergedFlow}
				<div class="h-full overflow-hidden">
					<FlowGraphV2
						modules={mergedFlow.modules}
						failureModule={mergedFlow.failure_module}
						preprocessorModule={mergedFlow.preprocessor_module}
						earlyStop={mergedFlow.skip_expr !== undefined}
						cache={mergedFlow.cache_ttl !== undefined}
						moduleActions={unifiedActions}
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
