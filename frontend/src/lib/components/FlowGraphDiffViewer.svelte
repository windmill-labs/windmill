<script lang="ts">
	import type { OpenFlow } from '$lib/gen'
	import YAML from 'yaml'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { Alert, Button } from './common'
	import { computeFlowModuleDiff } from './flows/flowDiff'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { DiffIcon, Minus, Plus, SquareSplitHorizontal } from 'lucide-svelte'
	import type { Viewport } from '@xyflow/svelte'

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	interface Props {
		beforeYaml: string
		afterYaml: string
	}

	let { beforeYaml, afterYaml }: Props = $props()

	let parseError = $state<string | undefined>(undefined)
	let viewerWidth = $state(SIDE_BY_SIDE_MIN_WIDTH)
	let beforePaneSize = $state(50)
	let viewMode = $state<'sidebyside' | 'unified'>('sidebyside')

	// Shared viewport for synchronizing both graphs in side-by-side mode
	let sharedViewport = $state<Viewport>({ x: 0, y: 0, zoom: 1 })

	let beforeGraph: FlowGraphV2 | undefined = $state(undefined)
	let afterGraph: FlowGraphV2 | undefined = $state(undefined)

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

	// Determine if we should render side-by-side or unified (user controlled via toggle)
	let isSideBySide = $derived(viewMode === 'sidebyside')

	// Build timeline using history-based approach
	// In side-by-side view, mark removed modules as 'shadowed' in the After graph
	// In unified view, mark removed modules as 'removed' to show them in red
	const { beforeActions } = $derived.by(() => {
		if (!beforeFlow || !afterFlow) return { beforeActions: undefined }
		return computeFlowModuleDiff(beforeFlow.value, afterFlow.value)
	})

	// Handler for viewport changes - updates shared state for synchronization
	function handleViewportChange(viewport: Viewport, isUserInitiated: boolean) {
		if (isUserInitiated) {
			sharedViewport = viewport
		}
	}

	$effect(() => {
		if (viewerWidth < SIDE_BY_SIDE_MIN_WIDTH) {
			viewMode = 'unified'
		} else {
			viewMode = 'sidebyside'
		}
	})
</script>

{#if parseError}
	<Alert type="error" title="Parse Error">
		{parseError}
	</Alert>
{:else if beforeFlow && afterFlow}
	<div class="h-full flex flex-col" bind:clientWidth={viewerWidth}>
		<!-- Header with view toggle -->
		<div class="flex flex-row items-center justify-end m-2 gap-4">
			<div>
				<ToggleButtonGroup bind:selected={viewMode}>
					{#snippet children({ item })}
						<ToggleButton {item} value="unified" label="Unified" icon={DiffIcon} />
						<ToggleButton
							{item}
							value="sidebyside"
							label="Side by Side"
							icon={SquareSplitHorizontal}
						/>
					{/snippet}
				</ToggleButtonGroup>
			</div>
			<!-- Header with controls and view toggle -->
			{#if isSideBySide}
				<!-- Shared controls for both graphs in side-by-side mode -->
				<div class="flex">
					<Button
						size="xs"
						color="light"
						variant="border"
						onClick={() => {
							beforeGraph?.zoomIn()
							afterGraph?.zoomIn()
						}}
						iconOnly
						startIcon={{ icon: Plus }}
					/>
					<Button
						size="xs"
						color="light"
						variant="border"
						onClick={() => {
							beforeGraph?.zoomOut()
							afterGraph?.zoomOut()
						}}
						iconOnly
						startIcon={{ icon: Minus }}
					/>
				</div>
			{/if}
		</div>

		<!-- Main content area -->
		<div class="flex-1 overflow-hidden">
			{#if isSideBySide}
				<!-- Side-by-side view for wide screens -->
				<Splitpanes class="!overflow-visible h-full">
					<!-- Before (Left) -->
					<Pane bind:size={beforePaneSize} minSize={30}>
						<div class="flex flex-col h-full border-r border-gray-200 dark:border-gray-700">
							<div class="flex-1 overflow-hidden">
								<FlowGraphV2
									bind:this={beforeGraph}
									modules={beforeFlow.value.modules}
									failureModule={beforeFlow.value.failure_module}
									preprocessorModule={beforeFlow.value.preprocessor_module}
									earlyStop={beforeFlow.value.skip_expr !== undefined}
									cache={beforeFlow.value.cache_ttl !== undefined}
									moduleActions={beforeActions}
									notSelectable={true}
									insertable={false}
									editMode={false}
									download={false}
									scroll={false}
									minHeight={400}
									triggerNode={false}
									{sharedViewport}
									onViewportChange={handleViewportChange}
								>
									{#snippet leftHeader()}
										<span class="text-sm text-primary">Before</span>
									{/snippet}
								</FlowGraphV2>
							</div>
						</div>
					</Pane>

					<!-- After (Right) - Show merged flow with shadowed removed modules -->
					<Pane minSize={30} class="flex flex-col h-full">
						<div class="flex flex-col h-full">
							<div class="flex-1 overflow-hidden">
								<FlowGraphV2
									bind:this={afterGraph}
									diffBeforeFlow={beforeFlow}
									modules={afterFlow.value.modules}
									failureModule={afterFlow.value.failure_module}
									preprocessorModule={afterFlow.value.preprocessor_module}
									earlyStop={afterFlow.value.skip_expr !== undefined}
									cache={afterFlow.value.cache_ttl !== undefined}
									currentInputSchema={afterFlow.schema}
									markRemovedAsShadowed={true}
									notSelectable={true}
									insertable={false}
									editMode={false}
									download={false}
									scroll={false}
									minHeight={400}
									triggerNode={false}
									{sharedViewport}
									onViewportChange={handleViewportChange}
								>
									{#snippet leftHeader()}
										<span class="text-sm text-primary">After</span>
									{/snippet}
								</FlowGraphV2>
							</div>
						</div>
					</Pane>
				</Splitpanes>
			{:else}
				<!-- Unified view - uses FlowGraphV2's built-in diff mode -->
				<div class="h-full overflow-hidden">
					<FlowGraphV2
						diffBeforeFlow={beforeFlow}
						modules={afterFlow.value.modules}
						failureModule={afterFlow.value.failure_module}
						preprocessorModule={afterFlow.value.preprocessor_module}
						earlyStop={afterFlow.value.skip_expr !== undefined}
						cache={afterFlow.value.cache_ttl !== undefined}
						currentInputSchema={afterFlow.schema}
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
		</div>
	</div>
{:else}
	<div class="flex items-center justify-center h-full">
		<p class="text-gray-500">Loading graphs...</p>
	</div>
{/if}
