<script lang="ts">
	import { FlowService, type FlowModule, type Job, type OpenFlow } from '../../gen'
	import { NODE, type GraphModuleState } from '.'
	import { getContext, onDestroy, setContext, tick, untrack, type Snippet } from 'svelte'
	import type { FlowGraphContext } from '../flows/types'
	import { createFlowDiffManager } from '../flows/flowDiffManager.svelte'

	import { get, writable, type Writable } from 'svelte/store'
	import '@xyflow/svelte/dist/base.css'
	import {
		SvelteFlow,
		type Node,
		type Edge,
		ConnectionLineType,
		Controls,
		ControlButton,
		SvelteFlowProvider,
		type Viewport
	} from '@xyflow/svelte'
	import {
		graphBuilder,
		isTriggerStep,
		topologicalSort,
		type InlineScript,
		type InsertKind,
		type NodeLayout,
		type onSelectedIteration,
		type SimplifiableFlow
	} from './graphBuilder.svelte'
	import ModuleNode from './renderers/nodes/ModuleNode.svelte'
	import InputNode from './renderers/nodes/InputNode.svelte'
	import BranchAllStart from './renderers/nodes/BranchAllStart.svelte'
	import BranchAllEndNode from './renderers/nodes/BranchAllEndNode.svelte'
	import ForLoopEndNode from './renderers/nodes/ForLoopEndNode.svelte'
	import ForLoopStartNode from './renderers/nodes/ForLoopStartNode.svelte'
	import ResultNode from './renderers/nodes/ResultNode.svelte'
	import BaseEdge from './renderers/edges/BaseEdge.svelte'
	import EmptyEdge from './renderers/edges/EmptyEdge.svelte'
	import { sugiyama, dagStratify, coordCenter, decrossTwoLayer, decrossOpt } from 'd3-dag'
	import { Expand } from 'lucide-svelte'
	import Toggle from '../Toggle.svelte'
	import DataflowEdge from './renderers/edges/DataflowEdge.svelte'
	import { encodeState, readFieldsRecursively } from '$lib/utils'
	import BranchOneStart from './renderers/nodes/BranchOneStart.svelte'
	import NoBranchNode from './renderers/nodes/NoBranchNode.svelte'
	import HiddenBaseEdge from './renderers/edges/HiddenBaseEdge.svelte'
	import TriggersNode from './renderers/nodes/TriggersNode.svelte'
	import { Alert, Drawer } from '../common'
	import Button from '../common/button/Button.svelte'
	import FlowYamlEditor from '../flows/header/FlowYamlEditor.svelte'
	import BranchOneEndNode from './renderers/nodes/branchOneEndNode.svelte'
	import type { TriggerContext } from '../triggers'
	import { workspaceStore } from '$lib/stores'
	import SubflowBound from './renderers/nodes/SubflowBound.svelte'
	import DiffDrawer from '../DiffDrawer.svelte'
	import ViewportResizer from './ViewportResizer.svelte'
	import ViewportSynchronizer from './ViewportSynchronizer.svelte'
	import AssetNode, { computeAssetNodes } from './renderers/nodes/AssetNode.svelte'
	import AssetsOverflowedNode from './renderers/nodes/AssetsOverflowedNode.svelte'
	import type { FlowGraphAssetContext } from '../flows/types'
	import AiToolNode, { computeAIToolNodes } from './renderers/nodes/AIToolNode.svelte'
	import NewAiToolNode from './renderers/nodes/NewAIToolNode.svelte'
	import { ChangeTracker } from '$lib/svelte5Utils.svelte'
	import type { ModulesTestStates } from '../modulesTest.svelte'
	import { deepEqual } from 'fast-equals'
	import type { AssetWithAltAccessType } from '../assets/lib'
	import type { ModuleActionInfo } from '../copilot/chat/flow/core'

	let useDataflow: Writable<boolean | undefined> = writable<boolean | undefined>(false)
	let showAssets: Writable<boolean | undefined> = writable<boolean | undefined>(true)

	const triggerContext = getContext<TriggerContext>('TriggerContext')

	// Create diffManager instance for this FlowGraphV2
	const diffManager = createFlowDiffManager()

	let fullWidth = 0
	let width = $state(0)

	let simplifiableFlow: SimplifiableFlow | undefined = $state(undefined)

	interface Props {
		success?: boolean | undefined
		modules?: FlowModule[] | undefined
		failureModule?: FlowModule | undefined
		preprocessorModule?: FlowModule | undefined
		minHeight?: number
		maxHeight?: number | undefined
		notSelectable?: boolean
		flowModuleStates?: Record<string, GraphModuleState> | undefined
		testModuleStates?: ModulesTestStates
		moduleActions?: Record<string, ModuleActionInfo>
		selectedId?: Writable<string | undefined>
		path?: string | undefined
		newFlow?: boolean
		insertable?: boolean
		earlyStop?: boolean
		cache?: boolean
		scroll?: boolean
		moving?: string | undefined
		// Download: display a top level button to open the graph in a new tab
		download?: boolean
		fullSize?: boolean
		disableAi?: boolean
		triggerNode?: boolean
		workspace?: string
		editMode?: boolean
		allowSimplifiedPoll?: boolean
		expandedSubflows?: Record<string, FlowModule[]>
		isOwner?: boolean
		isRunning?: boolean
		individualStepTests?: boolean
		flowJob?: Job | undefined
		showJobStatus?: boolean
		suspendStatus?: Record<string, { job: Job; nb: number }>
		chatInputEnabled?: boolean
		onDelete?: (id: string) => void
		onInsert?: (detail: {
			sourceId?: string
			targetId?: string
			branch?: { rootId: string; branch: number }
			index: number
			detail: string
			isPreprocessor?: boolean
			agentId?: string
			inlineScript?: InlineScript
			script?: { path: string; summary: string; hash: string | undefined }
			flow?: { path: string; summary: string }
			kind: InsertKind
		}) => Promise<void>
		onNewBranch?: (id: string) => Promise<void>
		onSelect?: (id: string | FlowModule) => void
		onDeleteBranch?: (detail: { id: string; index: number }) => Promise<void>
		onChangeId?: (detail: { id: string; newId: string; deps: Record<string, string[]> }) => void
		onMove?: (id: string) => void
		onUpdateMock?: (detail: { mock: FlowModule['mock']; id: string }) => void
		onTestUpTo?: ((id: string) => void) | undefined
		onSelectedIteration?: onSelectedIteration
		onEditInput?: (moduleId: string, key: string) => void
		onTestFlow?: () => void
		onCancelTestFlow?: () => void
		onOpenPreview?: () => void
		onHideJobStatus?: () => void
		flowHasChanged?: boolean
		// Viewport synchronization props (for diff viewer)
		sharedViewport?: Viewport
		onViewportChange?: (viewport: Viewport, isUserInitiated: boolean) => void
		leftHeader?: Snippet
		// Diff mode props
		diffBeforeFlow?: OpenFlow
		currentInputSchema?: Record<string, any>
		markRemovedAsShadowed?: boolean
	}

	let {
		onInsert = undefined,
		onDelete = undefined,
		onMove = undefined,
		onDeleteBranch = undefined,
		onNewBranch = undefined,
		onSelect = undefined,
		onChangeId = undefined,

		onUpdateMock = undefined,
		onSelectedIteration = undefined,
		success = undefined,
		modules = [],
		failureModule = undefined,
		preprocessorModule = undefined,
		minHeight = 0,
		maxHeight = undefined,
		notSelectable = false,
		flowModuleStates = undefined,
		testModuleStates = undefined,
		moduleActions = undefined,
		selectedId = writable<string | undefined>(undefined),
		path = undefined,
		newFlow = false,
		insertable = false,
		earlyStop = false,
		cache = false,
		scroll = false,
		moving = undefined,
		download = false,
		fullSize = false,
		disableAi = false,
		triggerNode = false,
		workspace = $workspaceStore ?? 'NO_WORKSPACE',
		editMode = false,
		allowSimplifiedPoll = true,
		expandedSubflows = $bindable({}),
		onTestUpTo = undefined,
		onEditInput = undefined,
		isOwner = false,
		onTestFlow = undefined,
		isRunning = false,
		onCancelTestFlow = undefined,
		onOpenPreview = undefined,
		onHideJobStatus = undefined,
		individualStepTests = false,
		flowJob = undefined,
		showJobStatus = false,
		suspendStatus = {},
		flowHasChanged = false,
		chatInputEnabled = false,
		sharedViewport = undefined,
		onViewportChange = undefined,
		leftHeader = undefined,
		diffBeforeFlow = undefined,
		currentInputSchema = undefined,
		markRemovedAsShadowed = false
	}: Props = $props()

	setContext<FlowGraphContext>('FlowGraphContext', { selectedId, useDataflow, showAssets, diffManager })

	// Validation: error if both diffBeforeFlow and moduleActions are provided
	$effect(() => {
		if (diffBeforeFlow && moduleActions) {
			throw new Error('Cannot provide both diffBeforeFlow and moduleActions props to FlowGraphV2')
		}
	})

	// Sync props to diffManager
	$effect(() => {
		if (diffBeforeFlow) {
			// Set snapshot from diffBeforeFlow
			diffManager.setSnapshot(diffBeforeFlow)
			diffManager.setInputSchemas(diffBeforeFlow.schema, currentInputSchema)
			diffManager.setMarkRemovedAsShadowed(markRemovedAsShadowed)

			// Set afterFlow from current modules
			const afterFlowValue = {
				modules: modules,
				failure_module: failureModule,
				preprocessor_module: preprocessorModule,
				skip_expr: earlyStop ? '' : undefined,
				cache_ttl: cache ? 300 : undefined
			}
			diffManager.setAfterFlow(afterFlowValue)
		} else if (moduleActions) {
			// Display-only mode: just set the module actions
			diffManager.setModuleActions(moduleActions)
		} else {
			// No diff mode: clear everything
			diffManager.clearSnapshot()
		}
	})

	// Watch current flow changes and update afterFlow for diff computation
	// This enables the diff visualization when flowStore is directly modified
	$effect(() => {
		// Only update if we have a snapshot (in diff mode) and no external diffBeforeFlow
		if (diffManager.beforeFlow && !diffBeforeFlow) {
			const afterFlowValue = {
				modules: modules,
				failure_module: failureModule,
				preprocessor_module: preprocessorModule,
				skip_expr: earlyStop ? '' : undefined,
				cache_ttl: cache ? 300 : undefined
			}
			diffManager.setAfterFlow(afterFlowValue)
			diffManager.setInputSchemas(diffManager.beforeFlow.schema, currentInputSchema)
		}
	})

	if (triggerContext && allowSimplifiedPoll) {
		if (isSimplifiable(modules)) {
			triggerContext?.simplifiedPoll?.set(true)
		}
		triggerContext?.simplifiedPoll.subscribe((value) => {
			computeSimplifiableFlow(modules ?? [], value ?? false)
		})
	}

	function computeSimplifiableFlow(modules: FlowModule[], simplifiedFlow: boolean) {
		const isSimplif = isSimplifiable(modules)
		simplifiableFlow = isSimplif ? { simplifiedFlow } : undefined
	}

	onDestroy(() => {
		if (isSimplifiable(modules)) {
			triggerContext?.simplifiedPoll?.set(undefined)
		}
	})

	function onModulesChange(modules: FlowModule[]) {
		computeSimplifiableFlow(
			modules,
			triggerContext?.simplifiedPoll ? (get(triggerContext.simplifiedPoll) ?? false) : false
		)
	}

	type NodeDep = { id: string; parentIds?: string[]; offset?: number }
	type NodePos = { position: { x: number; y: number } }
	let lastNodes: [NodeDep[], (NodeDep & NodePos)[]] | undefined = undefined
	function layoutNodes(nodes: NodeDep[]): (NodeDep & NodePos)[] {
		let lastResult = lastNodes?.[1]
		if (lastResult && deepEqual(nodes, lastNodes?.[0])) {
			console.debug('layoutNodes', 'same nodes')
			return lastResult
		}
		console.debug('layoutNodes', nodes.length)
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}

		let nodeWidths: Record<string, number> = {}
		const nodes2: (NodeDep & NodePos)[] = nodes.map((n) => {
			return { ...n, position: { x: 0, y: 0 } }
		})
		for (const n of topologicalSort(nodes)) {
			const endId = n.id + '-end'

			if (nodeWidths[endId] != undefined) {
				nodeWidths[n.id] = Math.max(nodeWidths[n.id] ?? 0, nodeWidths[endId])
			}
			if (n.parentIds && n.parentIds?.length == 1) {
				const parent = n.parentIds[0]
				const nodeWidth = nodeWidths[n.id] ?? 1
				nodeWidths[parent] = (nodeWidths[parent] ?? 0) + nodeWidth
			}
		}

		const dag = dagStratify().id(({ id }: NodeDep & NodePos) => id)(nodes2)

		let boxSize: any
		try {
			const layout = sugiyama()
				.decross(nodes.length > 20 ? decrossTwoLayer() : decrossOpt())
				.coord(coordCenter())
				.nodeSize((d) => {
					return [
						(nodeWidths[d?.data?.['id'] ?? ''] ?? 1) * (NODE.width + NODE.gap.horizontal * 1),
						NODE.height + NODE.gap.vertical
					] as readonly [number, number]
				})
			boxSize = layout(dag as any)
		} catch {
			const layout = sugiyama()
				.decross(decrossTwoLayer())
				.coord(coordCenter())
				.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
			boxSize = layout(dag as any)
		}

		const yOffset = insertable ? 100 : 0
		const newNodes = dag.descendants().map((des) => ({
			id: des.data.id,
			position: {
				x: des.x
					? // @ts-ignore
						(des.data.offset ?? 0) +
						// @ts-ignore
						des.x +
						(fullSize ? fullWidth : width) / 2 -
						boxSize.width / 2 -
						NODE.width / 2 -
						(width - fullWidth) / 2
					: 0,
				y: (des.y || 0) + yOffset
			}
		}))

		lastNodes = [nodes, newNodes]
		return newNodes
	}

	let eventHandler = {
		deleteBranch: (detail, label) => {
			$selectedId = label
			onDeleteBranch?.(detail)
		},
		insert: (detail) => {
			onInsert?.(detail)
		},
		select: (modId) => {
			if (!notSelectable) {
				if ($selectedId != modId) {
					$selectedId = modId
				}
				onSelect?.(modId)
			}
		},
		changeId: (detail) => {
			onChangeId?.(detail)
		},
		delete: (detail) => {
			onDelete?.(detail.id)
		},
		newBranch: (id) => {
			onNewBranch?.(id)
		},
		move: (detail) => {
			onMove?.(detail.id)
		},
		selectedIteration: (detail) => {
			onSelectedIteration?.(detail)
		},
		simplifyFlow: (detail) => {
			triggerContext?.simplifiedPoll.set(detail)
		},
		expandSubflow: async (id: string, path: string) => {
			const flow = await FlowService.getFlowByPath({ workspace: workspace, path })
			expandedSubflows[id] = flow.value.modules
			expandedSubflows = expandedSubflows
		},
		minimizeSubflow: (id: string) => {
			delete expandedSubflows[id]
			expandedSubflows = expandedSubflows
		},
		updateMock: (detail) => {
			onUpdateMock?.(detail)
		},
		testUpTo: (id: string) => {
			onTestUpTo?.(id)
		},
		editInput: (moduleId: string, key: string) => {
			onEditInput?.(moduleId, key)
		},
		testFlow: () => {
			onTestFlow?.()
		},
		cancelTestFlow: () => {
			onCancelTestFlow?.()
		},
		openPreview: () => {
			onOpenPreview?.()
		},
		hideJobStatus: () => {
			onHideJobStatus?.()
		}
	}

	// Use diffManager state for rendering
	let effectiveModuleActions = $derived(diffManager.moduleActions)

	let effectiveInputSchemaModified = $derived(
		effectiveModuleActions['Input']?.action === 'modified'
	)

	// Use merged flow when in diff mode (includes removed modules), otherwise use raw modules
	let effectiveModules = $derived(diffManager.mergedFlow?.modules ?? modules)

	let effectiveFailureModule = $derived(diffManager.mergedFlow?.failure_module ?? failureModule)

	let effectivePreprocessorModule = $derived(
		diffManager.mergedFlow?.preprocessor_module ?? preprocessorModule
	)

	// Initialize moduleTracker with effectiveModules
	let moduleTracker = $state(new ChangeTracker<FlowModule[]>([]))

	$inspect('HERE', effectiveModules)
	$inspect('HERE', effectiveModuleActions)
	$inspect('HERE', diffBeforeFlow)

	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])

	let height = $state(0)

	function isSimplifiable(modules: FlowModule[] | undefined): boolean {
		if (!modules || modules?.length !== 2) {
			return false
		}
		if (isTriggerStep(modules?.[0])) {
			let secondValue = modules?.[1].value
			return secondValue.type == 'forloopflow'
		}

		return false
	}

	async function updateStores() {
		if (graph.error) {
			return
		}
		// console.log('compute')

		let layoutedNodes = layoutNodes(
			Object.values(graph.nodes).map((n) => ({
				id: n.id,
				parentIds: n.parentIds,
				offset: n.data.offset ?? 0
			}))
		)
		let newNodes: (Node & NodeLayout)[] = layoutedNodes.map((n) => ({ ...n, ...graph.nodes[n.id] }))

		let assetNodesResult = $showAssets
			? computeAssetNodes(
					newNodes.map((n) => ({
						data: { assets: n.data?.assets as AssetWithAltAccessType[] },
						id: n.id,
						position: n.position
					}))
				)
			: undefined
		if (assetNodesResult) {
			newNodes = newNodes.map((n) => ({
				...n,
				position: assetNodesResult.newNodePositions[n.id]
			}))
		}
		let aiToolNodesResult = computeAIToolNodes(newNodes, eventHandler, insertable, flowModuleStates)
		nodes = [
			...newNodes.map((n) => ({ ...n, position: aiToolNodesResult.newNodePositions[n.id] })),
			...(assetNodesResult?.newAssetNodes ?? []),
			...aiToolNodesResult.toolNodes
		]
		edges = [
			...(assetNodesResult?.newAssetEdges ?? []),
			...aiToolNodesResult.toolEdges,
			...graph.edges
		]

		await tick()
		height = Math.max(...nodes.map((n) => n.position.y + NODE.height + 100), minHeight)
	}

	const nodeTypes = {
		input2: InputNode,
		module: ModuleNode,
		branchAllStart: BranchAllStart,
		branchAllEnd: BranchAllEndNode,
		forLoopEnd: ForLoopEndNode,
		forLoopStart: ForLoopStartNode,
		result: ResultNode,
		whileLoopStart: ForLoopStartNode,
		whileLoopEnd: ForLoopEndNode,
		branchOneStart: BranchOneStart,
		branchOneEnd: BranchOneEndNode,
		subflowBound: SubflowBound,
		noBranch: NoBranchNode,
		trigger: TriggersNode,
		asset: AssetNode,
		assetsOverflowed: AssetsOverflowedNode,
		aiTool: AiToolNode,
		newAiTool: NewAiToolNode
	} as any

	const edgeTypes = {
		edge: BaseEdge,
		empty: EmptyEdge,
		dataflowedge: DataflowEdge,
		hiddenedge: HiddenBaseEdge
	} as any

	const proOptions = { hideAttribution: true }

	// onMount(() => {
	// 	centerViewport(width)
	// })
	let yamlEditorDrawer: Drawer | undefined = $state(undefined)
	let diffDrawer: DiffDrawer | undefined = $state(undefined)

	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

	$effect(() => {
		allowSimplifiedPoll && modules && untrack(() => onModulesChange(modules ?? []))
	})
	$effect(() => {
		readFieldsRecursively(effectiveModules)
		untrack(() => moduleTracker.track($state.snapshot(effectiveModules)))
	})

	// Wire up the diff drawer to the diffManager
	$effect(() => {
		diffManager.setDiffDrawer(diffDrawer)
	})

	let graph = $derived.by(() => {
		console.log('HERE graph', effectiveModuleActions)
		moduleTracker.counter
		return graphBuilder(
			untrack(() => effectiveModules),
			{
				disableAi,
				insertable,
				flowModuleStates: untrack(() => flowModuleStates),
				testModuleStates: untrack(() => testModuleStates),
				moduleActions: effectiveModuleActions,
				inputSchemaModified: untrack(() => effectiveInputSchemaModified),
				selectedId: untrack(() => $selectedId),
				path,
				newFlow,
				cache,
				earlyStop,
				editMode,
				isOwner,
				isRunning,
				individualStepTests,
				flowJob,
				showJobStatus,
				suspendStatus,
				flowHasChanged,
				chatInputEnabled,
				additionalAssetsMap: flowGraphAssetsCtx?.val.additionalAssetsMap
			},
			untrack(() => effectiveFailureModule),
			effectivePreprocessorModule,
			eventHandler,
			success,
			$useDataflow,
			untrack(() => $selectedId),
			moving,
			simplifiableFlow,
			triggerNode ? path : undefined,
			expandedSubflows
		)
	})
	let hideAssetsToggle = $derived(
		$showAssets && Object.values(nodes).every((n) => n.type !== 'asset')
	)

	$effect(() => {
		;[graph, allowSimplifiedPoll, $showAssets]
		untrack(() => updateStores())
	})

	let showDataflow = $derived(
		$selectedId != undefined &&
			!$selectedId.startsWith('constants') &&
			!$selectedId.startsWith('settings') &&
			$selectedId !== 'failure' &&
			$selectedId !== 'preprocessor' &&
			$selectedId !== 'Result' &&
			$selectedId !== 'triggers'
	)
	let debouncedWidth: number | undefined = $state(undefined)
	let timeout: number | undefined = $state(undefined)
	$effect(() => {
		if (!debouncedWidth) {
			return
		}
		if (untrack(() => width) == undefined) {
			width = debouncedWidth
			return
		}
		if (untrack(() => timeout)) {
			clearTimeout(untrack(() => timeout))
		}
		timeout = setTimeout(() => {
			if (debouncedWidth && untrack(() => width) != debouncedWidth) {
				width = debouncedWidth
			}
		}, 10)
	})

	let viewportResizer: ViewportResizer | undefined = $state(undefined)
	let viewportSynchronizer: ViewportSynchronizer | undefined = $state(undefined)

	export function isNodeVisible(nodeId: string): boolean {
		return viewportResizer?.isNodeVisible(nodeId) ?? false
	}

	export function zoomIn() {
		viewportSynchronizer?.zoomIn()
	}

	export function zoomOut() {
		viewportSynchronizer?.zoomOut()
	}

	export function getDiffManager() {
		return diffManager
	}

	$inspect('HERE effectiveModuleActions', effectiveModuleActions)
</script>

{#if insertable}
	<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
{/if}
<DiffDrawer bind:this={diffDrawer} />
<div
	style={`height: ${height}px; max-height: ${maxHeight}px;`}
	class="overflow-clip"
	bind:clientWidth={debouncedWidth}
>
	{#if graph?.error}
		<div class="center-center p-2">
			<Alert title="Error parsing the flow" type="error" class="max-w-1/2">
				{graph.error}

				<Button
					color="red"
					size="xs"
					btnClasses="mt-2 w-min"
					on:click={() => yamlEditorDrawer?.openDrawer()}>Open YAML editor</Button
				>
			</Alert>
		</div>
	{:else}
		<SvelteFlowProvider>
			<ViewportResizer {height} {width} {nodes} bind:this={viewportResizer} />
			{#if sharedViewport && onViewportChange}
				<ViewportSynchronizer
					{sharedViewport}
					onLocalChange={onViewportChange}
					bind:this={viewportSynchronizer}
				/>
			{/if}
			<SvelteFlow
				onpaneclick={(e) => {
					document.dispatchEvent(new Event('focus'))
				}}
				onmove={(event, viewport) => {
					viewportSynchronizer?.handleLocalViewportChange(event, viewport)
				}}
				{nodes}
				{edges}
				{edgeTypes}
				{nodeTypes}
				{height}
				{width}
				minZoom={0.2}
				maxZoom={1.2}
				connectionLineType={ConnectionLineType.SmoothStep}
				defaultEdgeOptions={{ type: 'smoothstep' }}
				preventScrolling={scroll}
				zoomOnDoubleClick={false}
				elementsSelectable={false}
				{proOptions}
				nodesDraggable={false}
				--background-color={false}
			>
				<div class="absolute inset-0 !bg-surface-secondary h-full" id="flow-graph-v2"></div>
				{#if leftHeader}
					<div class="absolute top-2 left-2 z-10">
						{@render leftHeader()}
					</div>
				{:else}
					<Controls position="top-right" orientation="horizontal" showLock={false}>
						{#if download}
							<ControlButton
								onclick={() => {
									try {
										localStorage.setItem(
											'svelvet',
											encodeState({ modules, failureModule, preprocessorModule })
										)
									} catch (e) {
										console.error('error interacting with local storage', e)
									}
									window.open('/view_graph', '_blank')
								}}
								class="!bg-surface"
							>
								<Expand size="14" />
							</ControlButton>
						{/if}
					</Controls>

					<Controls
						position="top-left"
						orientation="vertical"
						showLock={false}
						showZoom={false}
						showFitView={false}
						class="!shadow-none gap-3"
						style={leftHeader ? 'margin-top: 40px;' : ''}
					>
						{#if !hideAssetsToggle}
							<Toggle bind:checked={$showAssets} size="xs" options={{ right: 'Assets' }} />
						{/if}
						{#if showDataflow}
							<Toggle bind:checked={$useDataflow} size="xs" options={{ right: 'Dataflow' }} />
						{/if}
					</Controls>
				{/if}
			</SvelteFlow>
		</SvelteFlowProvider>
	{/if}
</div>

<style lang="postcss">
	:global(.svelte-flow__handle) {
		opacity: 0;
	}

	:global(.svelte-flow__controls-button) {
		@apply bg-surface border-0;
	}
	:global(.svelte-flow__controls-button:hover) {
		@apply bg-surface-hover;
	}

	:global(.svelte-flow__edgelabel-renderer) {
		@apply z-50;
	}
</style>
