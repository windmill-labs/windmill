<script lang="ts">
	import type { OpenFlow } from '$lib/gen'
	import YAML from 'yaml'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { Alert } from './common'
	import { computeFlowModuleDiff } from './flows/flowDiff'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { DiffIcon, Minus, Plus, SquareSplitHorizontal } from 'lucide-svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import type { Viewport } from '@xyflow/svelte'

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	interface Props {
		beforeYaml: string
		afterYaml: string
		/** When true, render an empty surface placeholder for the "before"
		 * pane in side-by-side mode (use for added items where there's no
		 * prior flow to show). */
		beforeMissing?: boolean
		/** Same as `beforeMissing` but for the "after" pane (use for removed
		 * items). */
		afterMissing?: boolean
		/** Render the unified single-pane diff when true, side-by-side
		 * otherwise. When undefined, the component renders its own
		 * Unified / Side-by-side toggle in the corner (legacy behavior for
		 * the standalone comparison page). A narrow viewer still falls back
		 * to unified automatically. */
		inlineDiff?: boolean | undefined
	}

	let {
		beforeYaml,
		afterYaml,
		beforeMissing = false,
		afterMissing = false,
		inlineDiff = undefined
	}: Props = $props()

	// Local toggle state, used only when no inlineDiff prop is supplied.
	let localViewMode = $state<'sidebyside' | 'unified'>('sidebyside')
	const showLocalToggle = $derived(inlineDiff === undefined)
	const effectiveInlineDiff = $derived(
		inlineDiff !== undefined ? inlineDiff : localViewMode === 'unified'
	)

	let viewerWidth = $state(SIDE_BY_SIDE_MIN_WIDTH)
	let beforePaneSize = $state(50)
	// Track the content area's rendered height so unified-mode graphs can
	// grow to fill the diff box (otherwise FlowGraphV2 sits at its
	// content-fit height + small floor, leaving empty space below).
	let contentAreaHeight = $state(0)

	// Each FlowGraphV2 sizes itself to its own content (clamped to minHeight).
	// In side-by-side mode we want both graphs to share the same height, so
	// we track each side's reported height and feed back the max as minHeight
	// to both. The width-graph then stays at its computed size; the shorter
	// graph grows to match.
	let beforeContentHeight = $state(0)
	let afterContentHeight = $state(0)
	const SHARED_MIN_HEIGHT = 400
	const sharedMinHeight = $derived(
		Math.max(SHARED_MIN_HEIGHT, beforeContentHeight, afterContentHeight)
	)

	// Shared viewport for synchronizing both graphs in side-by-side mode
	let sharedViewport = $state<Viewport>({ x: 0, y: 0, zoom: 1 })

	let beforeGraph: FlowGraphV2 | undefined = $state(undefined)
	let afterGraph: FlowGraphV2 | undefined = $state(undefined)

	function parseFlow(
		yaml: string,
		label: 'before' | 'after'
	): {
		flow: OpenFlow | undefined
		error: string | undefined
	} {
		try {
			return { flow: YAML.parse(yaml) as OpenFlow, error: undefined }
		} catch (error) {
			return {
				flow: undefined,
				error: `Error parsing ${label} flow: ${
					error instanceof Error
						? error.message
						: typeof error === 'string'
							? error
							: 'Unknown error'
				}`
			}
		}
	}

	// For added/removed items, the caller passes empty YAML and sets the
	// corresponding *Missing flag. We swap in an empty OpenFlow stub on
	// that side so the unified diff path still has something to compare
	// against (every module on the present side becomes added / removed).
	// The side-by-side rendering uses the flag directly to draw a
	// placeholder pane instead.
	const EMPTY_FLOW: OpenFlow = { summary: '', value: { modules: [] } }

	let beforeParsed = $derived.by(() =>
		beforeMissing ? { flow: EMPTY_FLOW, error: undefined } : parseFlow(beforeYaml, 'before')
	)
	let afterParsed = $derived.by(() =>
		afterMissing ? { flow: EMPTY_FLOW, error: undefined } : parseFlow(afterYaml, 'after')
	)
	let parseError = $derived(beforeParsed.error ?? afterParsed.error)
	let beforeFlow: OpenFlow | undefined = $derived(beforeParsed.flow)
	let afterFlow: OpenFlow | undefined = $derived(afterParsed.flow)

	// Side-by-side unless the caller asked for unified, OR the viewer pane
	// is too narrow to comfortably split (fallback to unified for legibility).
	const isSideBySide = $derived(!effectiveInlineDiff && viewerWidth >= SIDE_BY_SIDE_MIN_WIDTH)

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
</script>

{#if parseError}
	<Alert type="error" title="Parse Error">
		{parseError}
	</Alert>
{:else if beforeFlow && afterFlow}
	<div class="h-full flex flex-col" bind:clientWidth={viewerWidth}>
		{#if showLocalToggle}
			<!-- Legacy top banner — only holds the local Unified / Side-by-side
			     toggle when the parent doesn't pre-set inlineDiff. Zoom
			     controls live as an overlay below (same as the controlled path). -->
			<div class="flex flex-row items-center justify-end m-2">
				<ToggleButtonGroup bind:selected={localViewMode} noWFull>
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
		{/if}
		<!-- Main content area -->
		<div class="flex-1 overflow-hidden relative" bind:clientHeight={contentAreaHeight}>
			{#if isSideBySide}
				<!-- Shared zoom controls overlay the graph viewport. Uses xy-flow's
				     own `.svelte-flow__controls` / `.svelte-flow__controls-button`
				     classes so it inherits the same look as FlowGraphV2's built-in
				     controls (FlowGraphV2's global override gives the buttons
				     bg-surface + hover bg-surface-hover with border:0). Local
				     overrides bump the icon size from 12px to 16px and drop the
				     xy-flow default shadow. No fit-view button — recenter can't
				     sync across two graphs. -->
				<div
					class="svelte-flow__controls horizontal absolute top-[15px] right-[15px] z-10 rounded bg-surface border border-gray-200 dark:border-gray-700 overflow-hidden diff-zoom-controls"
				>
					<button
						type="button"
						aria-label="Zoom in"
						class="svelte-flow__controls-button"
						onclick={() => {
							beforeGraph?.zoomIn()
							afterGraph?.zoomIn()
						}}
					>
						<Plus />
					</button>
					<button
						type="button"
						aria-label="Zoom out"
						class="svelte-flow__controls-button"
						onclick={() => {
							beforeGraph?.zoomOut()
							afterGraph?.zoomOut()
						}}
					>
						<Minus />
					</button>
				</div>
			{/if}
			{#if isSideBySide}
				<!-- Side-by-side view for wide screens -->
				<Splitpanes class="!overflow-visible h-full">
					<!-- Before (Left) -->
					<Pane bind:size={beforePaneSize} minSize={30}>
						<div
							class="flex flex-col h-full border-r border-gray-200 dark:border-gray-700 relative bg-surface-secondary {beforeMissing
								? 'hatched-thin'
								: ''}"
						>
							{#if beforeMissing}
								<span class="absolute top-2 left-2 z-10 text-2xs text-tertiary">
									Before <span class="italic">(no prior version)</span>
								</span>
							{:else}
								<div class="flex-1 overflow-hidden">
									<FlowGraphV2
										bind:this={beforeGraph}
										modules={beforeFlow.value.modules}
										groups={beforeFlow.value.groups}
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
										minHeight={sharedMinHeight}
										triggerNode={false}
										{sharedViewport}
										onViewportChange={handleViewportChange}
										onHeight={(h) => (beforeContentHeight = h)}
									>
										{#snippet leftHeader()}
											<span class="text-2xs text-tertiary">Before</span>
										{/snippet}
									</FlowGraphV2>
								</div>
							{/if}
						</div>
					</Pane>

					<!-- After (Right) - Show merged flow with shadowed removed modules -->
					<Pane minSize={30} class="flex flex-col h-full">
						<div
							class="flex flex-col h-full relative bg-surface-secondary {afterMissing
								? 'hatched-thin'
								: ''}"
						>
							{#if afterMissing}
								<span class="absolute top-2 left-2 z-10 text-2xs text-tertiary">
									After <span class="italic">(flow deleted)</span>
								</span>
							{:else}
								<div class="flex-1 overflow-hidden">
									<FlowGraphV2
										bind:this={afterGraph}
										diffBeforeFlow={beforeFlow}
										modules={afterFlow.value.modules}
										groups={afterFlow.value.groups}
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
										minHeight={sharedMinHeight}
										triggerNode={false}
										{sharedViewport}
										onViewportChange={handleViewportChange}
										onHeight={(h) => (afterContentHeight = h)}
									>
										{#snippet leftHeader()}
											<span class="text-2xs text-tertiary">After</span>
										{/snippet}
									</FlowGraphV2>
								</div>
							{/if}
						</div>
					</Pane>
				</Splitpanes>
			{:else}
				<!-- Unified view - uses FlowGraphV2's built-in diff mode -->
				<div class="h-full overflow-hidden">
					<FlowGraphV2
						diffBeforeFlow={beforeFlow}
						modules={afterFlow.value.modules}
						groups={afterFlow.value.groups}
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
						minHeight={Math.max(contentAreaHeight, SHARED_MIN_HEIGHT)}
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

<style>
	/* Thin diagonal hatch used as the empty-pane fill. */
	.hatched-thin {
		background-image: repeating-linear-gradient(
			-45deg,
			transparent 0,
			transparent 6px,
			rgba(128, 128, 128, 0.16) 6px,
			rgba(128, 128, 128, 0.16) 7.5px
		);
	}

	/* Shared zoom controls overlay: same xy-flow layout as the in-graph
	   controls (`.svelte-flow__controls.horizontal`) but with bigger Plus/Minus
	   glyphs (xy-flow caps svg at 12px by default) and no panel shadow. */
	.diff-zoom-controls {
		box-shadow: none !important;
	}
	.diff-zoom-controls :global(.svelte-flow__controls-button) {
		width: 28px;
		height: 28px;
	}
	.diff-zoom-controls :global(.svelte-flow__controls-button svg) {
		max-width: 16px;
		max-height: 16px;
	}
</style>
