<script lang="ts">
	import { FlowService, type FlowModule, type FlowNote, type Job, type OpenFlow } from '../../gen'
	import { AI_OR_ASSET_NODE_TYPES, NODE, type GraphModuleState } from '.'
	import { getContext, onDestroy, onMount, tick, untrack, type Snippet } from 'svelte'
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
		type Viewport,
		SelectionMode
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
	import { Expand, MousePointer, Hand } from 'lucide-svelte'
	import Toggle from '../Toggle.svelte'
	import DataflowEdge from './renderers/edges/DataflowEdge.svelte'
	import { encodeState, readFieldsRecursively, getModifierKey, isMac } from '$lib/utils'
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
	import NoteNode from './renderers/nodes/NoteNode.svelte'
	import NoteTool from './NoteTool.svelte'
	import SelectionBoundingBox from './SelectionBoundingBox.svelte'
	import SelectionTool from './SelectionTool.svelte'
	import PaneContextMenu from './PaneContextMenu.svelte'
	import { SelectionManager } from './selectionUtils.svelte'
	import { ChangeTracker } from '$lib/svelte5Utils.svelte'
	import { NoteManager } from './noteManager.svelte'
	import type { ModulesTestStates } from '../modulesTest.svelte'
	import { deepEqual } from 'fast-equals'
	import type { AssetWithAltAccessType } from '../assets/lib'
	import type { ModuleActionInfo } from '$lib/components/flows/flowDiff'
	import { setGraphContext } from './graphContext'
	import { computeNoteNodes } from './noteUtils.svelte'
	import { Tooltip } from '../meltComponents'
	import { getNoteEditorContext } from './noteEditor.svelte'

	let useDataflow: Writable<boolean | undefined> = writable<boolean | undefined>(false)
	let showAssets: Writable<boolean | undefined> = writable<boolean | undefined>(true)
	let showNotes = $state(true)

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
		selectionManager?: SelectionManager
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
		noteMode?: boolean
		notes?: FlowNote[]
		chatInputEnabled?: boolean
		multiSelectEnabled?: boolean
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
		exitNoteMode?: () => void
		onNotePositionUpdate?: (noteId: string, position: { x: number; y: number }) => void
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
		selectionManager: selectionManagerProp = undefined,
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
		noteMode = false,
		notes = undefined,
		exitNoteMode = undefined,
		onNotePositionUpdate = undefined,
		chatInputEnabled = false,
		sharedViewport = undefined,
		onViewportChange = undefined,
		leftHeader = undefined,
		diffBeforeFlow = undefined,
		currentInputSchema = undefined,
		markRemovedAsShadowed = false,
		multiSelectEnabled = false
	}: Props = $props()

	// Initialize note manager with fine-grained reactivity
	const noteManager = new NoteManager(
		() => notes ?? [],
		(newNodes) => {
			nodes = newNodes
		},
		() => nodes
	)

	// Runtime text height tracking for notes (not stored in FlowNote)
	let noteTextHeights = $state<Record<string, number>>({})

	// Reference to pane context menu component
	let paneContextMenu: PaneContextMenu | undefined = $state(undefined)
	let flowContainer: HTMLDivElement | undefined = $state(undefined)

	// Selection manager - create one if not provided
	let selectionManager = selectionManagerProp || new SelectionManager()
	const selectedId = $derived(selectionManager.getSelectedId())

	const noteEditorContext = getNoteEditorContext()

	// Function to calculate extra gap needed for notes below the lowest flow nodes
	function calculateNoteGap(notes: FlowNote[] | undefined): number {
		if (!notes || notes.length === 0) {
			return 0
		}
		let maxNoteBelowGap = 0

		notes.forEach((note) => {
			if (note.position?.y && note.position.y < 0) {
				maxNoteBelowGap = Math.max(maxNoteBelowGap, -note.position.y)
			}
		})

		return maxNoteBelowGap
	}

	// Calculate note gap based on current nodes and notes
	const topPadding = editMode ? 100 : 24
	const yOffset = calculateNoteGap(notes) + topPadding

	setGraphContext({
		selectionManager: selectionManager,
		useDataflow,
		showAssets,
		noteManager,
		clearFlowSelection,
		yOffset,
		diffManager
	} as any)

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
		diffManager.setDiffDrawer(undefined)
	})

	function onModulesChange(modules: FlowModule[]) {
		computeSimplifiableFlow(
			modules,
			triggerContext?.simplifiedPoll ? (get(triggerContext.simplifiedPoll) ?? false) : false
		)
	}

	type NodeDep = {
		id: string
		parentIds?: string[]
		offset?: number
		data?: { assets?: AssetWithAltAccessType[] }
	}
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
				y: des.y || 0
			}
		}))

		lastNodes = [nodes, newNodes]
		return newNodes
	}

	let eventHandler = {
		deleteBranch: (detail, label) => {
			selectionManager.selectId(label)
			onDeleteBranch?.(detail)
		},
		insert: (detail) => {
			onInsert?.(detail)
		},
		select: (modId) => {
			// AI tools are not selectable by the flow. Selection has to be refactored to be simplier.
			if (nodes.find((n) => n.data?.moduleId === modId)?.type === 'aiTool' || modId === 'Trigger') {
				selectionManager.selectId(modId)
			}
			if (!notSelectable) {
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

	// Validation: error if both diffBeforeFlow and moduleActions are provided
	$effect(() => {
		if (diffBeforeFlow && moduleActions) {
			throw new Error('Cannot provide both diffBeforeFlow and moduleActions props to FlowGraphV2')
		}
	})

	// Sync props to diffManager
	$effect(() => {
		const currentFlowValue = {
			modules: modules,
			failure_module: failureModule,
			preprocessor_module: preprocessorModule
		}
		diffManager.setCurrentFlow(currentFlowValue)
		diffManager.setCurrentInputSchema(currentInputSchema)

		// Handle diff mode setup
		if (diffBeforeFlow) {
			diffManager.setEditMode(editMode)
			diffManager.setBeforeFlow(diffBeforeFlow)
			diffManager.setMarkRemovedAsShadowed(markRemovedAsShadowed)
		} else if (moduleActions) {
			// Display-only mode: just set the module actions
			diffManager.setModuleActions(moduleActions)
		}
	})

	// Use diffManager state for rendering
	let effectiveModuleActions = $derived(diffManager.moduleActions)

	// Use merged flow when in diff mode (includes removed modules), otherwise use raw modules
	let effectiveModules = $derived(diffManager.mergedFlow?.modules ?? modules)

	let effectiveFailureModule = $derived(diffManager.mergedFlow?.failure_module ?? failureModule)

	let effectivePreprocessorModule = $derived(
		diffManager.mergedFlow?.preprocessor_module ?? preprocessorModule
	)

	let canUseDiffDrawer = $derived(diffBeforeFlow || moduleActions || editMode)

	// Initialize moduleTracker with effectiveModules
	let moduleTracker = $state(new ChangeTracker<FlowModule[]>([]))

	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])

	let height = $state(0)

	// Derived nodes with yOffset applied to all nodes uniformly and selectable flag set to false if notSelectable is true
	const nodesWithOffset = $derived.by(() => {
		return nodes.map((node) => {
			if (node.type && !AI_OR_ASSET_NODE_TYPES.includes(node.type)) {
				return {
					...node,
					position: { ...node.position, y: node.position.y + yOffset },
					selectable: notSelectable ? false : node.selectable
				}
			}
			return {
				...node,
				selectable: notSelectable ? false : node.selectable
			}
		})
	})

	// Note feature state

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

	// Clear SvelteFlow's internal selection by creating new nodes array
	function clearFlowSelection() {
		nodes = nodes.map((node) => {
			if (node.selected) {
				return { ...node, selected: false }
			}
			return node
		})
	}

	// Keyboard event handling
	function handleKeyDown(event: KeyboardEvent) {
		selectionManager.handleKeyDown(event)
		noteManager.handleKeyDown(event)
		if (event.key === 'Escape') {
			if (noteMode) {
				exitNoteMode?.()
			}
		}
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
				offset: n.data.offset ?? 0,
				data: { assets: (n.data as any).assets }
			}))
		)
		let newNodes: (Node & NodeLayout)[] = layoutedNodes.map((n) => ({ ...n, ...graph.nodes[n.id] }))

		let assetNodesResult = $showAssets
			? computeAssetNodes(
					newNodes.map((n) => ({
						data: { assets: n.data?.assets as AssetWithAltAccessType[], offset: n.data?.offset as number },
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
		let nodesAfterAITools = newNodes.map((n) => ({
			...n,
			position: aiToolNodesResult.newNodePositions[n.id]
		}))

		let finalNodes = [
			...nodesAfterAITools,
			...(assetNodesResult?.newAssetNodes ?? []),
			...aiToolNodesResult.toolNodes
		]

		// Compute note nodes and positions
		let noteNodesResult = showNotes
			? computeNoteNodes(
					finalNodes.map((n) => ({
						id: n.id,
						position: n.position,
						parentIds: n.parentIds,
						offset: n.data?.offset ?? 0,
						data: { assets: (n.data as any)?.assets },
						type: n.type
					})),
					notes ?? [],
					noteTextHeights,
					(noteId: string, height: number) => {
						noteTextHeights[noteId] = height
						noteManager.render()
					},
					editMode,
					noteEditorContext
				)
			: undefined

		// Apply note positioning to nodes if notes are enabled
		if (noteNodesResult) {
			finalNodes = finalNodes.map((n) => ({
				...n,
				position: noteNodesResult.newNodePositions[n.id] || n.position
			}))
		}

		// update nodes
		nodes = [...finalNodes, ...(noteNodesResult?.noteNodes ?? [])]

		edges = [
			...(assetNodesResult?.newAssetEdges ?? []),
			...aiToolNodesResult.toolEdges,
			...graph.edges
		]

		await tick()
		updateHeight()
	}

	function updateHeight() {
		if (nodes.length === 0) {
			height = minHeight
		} else {
			const minY = Math.min(...nodes.map((n) => n.position.y))
			const maxBottom = Math.max(...nodes.map((n) => n.position.y + NODE.height + 100))
			height = Math.max(maxBottom - minY, minHeight)
		}
	}

	$effect(() => {
		minHeight
		untrack(() => updateHeight())
	})

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
		newAiTool: NewAiToolNode,
		note: NoteNode
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
		moduleTracker.counter
		effectiveModuleActions
		return graphBuilder(
			untrack(() => effectiveModules),
			{
				disableAi,
				insertable,
				flowModuleStates: untrack(() => flowModuleStates),
				testModuleStates: untrack(() => testModuleStates),
				moduleActions: untrack(() => effectiveModuleActions),
				selectedId: untrack(() => selectedId),
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
			untrack(() => selectedId),
			moving,
			simplifiableFlow,
			triggerNode ? path : undefined,
			expandedSubflows
		)
	})
	let hideAssetsToggle = $derived(
		$showAssets && Object.values(nodes).every((n) => n.type !== 'asset')
	)
	let hideNotesToggle = $derived(!notes || notes.length === 0)

	$effect(() => {
		;[graph, allowSimplifiedPoll, $showAssets, showNotes, noteManager.renderCount]
		untrack(async () => {
			await updateStores()
		})
	})

	// Add global keyboard event listener for selection controls
	onMount(() => {
		function globalKeyDownHandler(event: KeyboardEvent) {
			handleKeyDown(event)
		}

		document.addEventListener('keydown', globalKeyDownHandler)

		return () => {
			document.removeEventListener('keydown', globalKeyDownHandler)
		}
	})

	// DOM event handling for pane clicks in rect-select mode
	$effect(() => {
		// Only add manual handling when in rect-select mode
		if (selectionManager.mode !== 'rect-select') {
			return
		}

		function paneClickHandler(event: Event) {
			// Find the pane within our specific flow container
			const pane = flowContainer?.querySelector('.svelte-flow__pane')
			if (!pane || !event.target || !pane.contains(event.target as Element)) {
				return
			}

			// Don't trigger if clicking on nodes or UI elements
			const target = event.target as Element
			if (
				target.closest('.svelte-flow__node') ||
				target.closest('button') ||
				target.closest('[role="button"]') ||
				target.closest('.svelte-flow__controls')
			) {
				return
			}

			// Trigger the same logic as onpaneclick
			document.dispatchEvent(new Event('focus'))
			selectionManager.clearSelection()
		}

		const pane = flowContainer?.querySelector('.svelte-flow__pane')
		if (pane) {
			pane.addEventListener('click', paneClickHandler)
		}

		return () => {
			const pane = flowContainer?.querySelector('.svelte-flow__pane')
			if (pane) {
				pane.removeEventListener('click', paneClickHandler)
			}
		}
	})

	let showDataflow = $derived(
		selectedId !== undefined &&
			selectedId !== null &&
			!selectedId?.startsWith('constants') &&
			!selectedId?.startsWith('settings') &&
			selectedId !== 'failure' &&
			selectedId !== 'preprocessor' &&
			selectedId !== 'Result' &&
			selectedId !== 'Trigger'
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

	export function enableNotes() {
		if (!showNotes) {
			showNotes = true
		}
	}

	const modifierKey = isMac() ? 'Meta' : 'Control'
</script>

{#if insertable}
	<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
{/if}
{#if canUseDiffDrawer}
	<DiffDrawer bind:this={diffDrawer} />
{/if}
<div
	style={`height: ${height}px; max-height: ${maxHeight}px;`}
	class="overflow-clip relative"
	bind:clientWidth={debouncedWidth}
	bind:this={flowContainer}
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
			<PaneContextMenu {editMode} bind:this={paneContextMenu} />
			<SvelteFlow
				onpaneclick={() => {
					document.dispatchEvent(new Event('focus'))
					selectionManager.clearSelection()
				}}
				onpanecontextmenu={({ event }) => {
					paneContextMenu?.onPaneContextMenu(event)
				}}
				onnodedragstop={(event) => {
					const node = event.targetNode
					if (node && node.type === 'note') {
						const positionWithOffset = {
							x: node.position.x,
							y: node.position.y - yOffset
						}
						onNotePositionUpdate?.(node.id, positionWithOffset)
					}
				}}
				onmove={(event, viewport) => {
					viewportSynchronizer?.handleLocalViewportChange(event, viewport)
				}}
				nodes={nodesWithOffset}
				{edges}
				{edgeTypes}
				{nodeTypes}
				{height}
				{width}
				minZoom={0.2}
				maxZoom={1.6}
				connectionLineType={ConnectionLineType.SmoothStep}
				defaultEdgeOptions={{ type: 'smoothstep' }}
				preventScrolling={scroll}
				selectionOnDrag={selectionManager.mode === 'rect-select'}
				elementsSelectable={true}
				selectionMode={SelectionMode.Partial}
				selectionKey={selectionManager.mode === 'rect-select' || !editMode ? null : modifierKey}
				panActivationKey={selectionManager.mode === 'rect-select' ? modifierKey : null}
				panOnDrag={selectionManager.mode === 'rect-select' ? [1] : true}
				zoomOnDoubleClick={false}
				elevateNodesOnSelect={false}
				{proOptions}
				multiSelectionKey={'Shift'}
				nodesDraggable={false}
				--background-color={false}
			>
				<div class="absolute inset-0 !bg-surface-secondary h-full" id="flow-graph-v2"></div>

				{#if noteMode}
					<NoteTool {exitNoteMode} {yOffset} />
				{/if}

				{#if multiSelectEnabled}
					<SelectionBoundingBox
						selectedNodes={selectionManager.selectedIds}
						allNodes={nodesWithOffset as (Node & { type: string })[]}
					/>
				{/if}

				<!-- SelectionTool for handling selection changes and filtering -->
				<SelectionTool {selectionManager} clearGraphSelection={clearFlowSelection} />

				{#if leftHeader}
					<div class="absolute top-2 left-2 z-10">
						{@render leftHeader()}
					</div>
				{:else}
					<Controls position="top-right" orientation="horizontal" showLock={false}>
						{#if multiSelectEnabled}
							<div class="flex items-center gap-2">
								<Tooltip>
									<ControlButton
										onclick={() => {
											selectionManager.mode =
												selectionManager.mode === 'normal' ? 'rect-select' : 'normal'
										}}
									>
										{#if selectionManager.mode === 'rect-select'}
											<MousePointer size="14" />
										{:else}
											<Hand size="14" />
										{/if}
									</ControlButton>
									{#snippet text()}
										<div class="flex flex-col gap-2">
											<div class="flex items-center gap-2">
												<Hand size="14" />
												<span class="text-secondary"
													><strong class="text-primary">Grab</strong>: Click and drag to pan. Hold
													<kbd class="text-primary text-lg">{getModifierKey()}</kbd> to box select.</span
												>
											</div>
											<div class="flex items-center gap-2">
												<MousePointer size="14" />
												<span class="text-secondary"
													><strong class="text-primary">Select</strong> Click and drag to box
													select. Hold
													<kbd class="text-primary text-lg">{getModifierKey()}</kbd> to pan.</span
												>
											</div>
										</div>
									{/snippet}
								</Tooltip>
							</div>
						{/if}
						{#if download}
							<ControlButton
								onclick={() => {
									try {
										localStorage.setItem(
											'svelvet',
											encodeState({ modules, failureModule, preprocessorModule, notes })
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
						{#if !hideNotesToggle}
							<Toggle bind:checked={showNotes} size="xs" options={{ right: 'Notes' }} />
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

	:global(.svelte-flow__selection) {
		display: none;
		pointer-events: none;
	}
</style>
