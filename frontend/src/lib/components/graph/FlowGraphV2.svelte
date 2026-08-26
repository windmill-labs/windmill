<script lang="ts">
	import GraphZoomControls from './GraphZoomControls.svelte'
	import FlowPanelPlacementPicker from '$lib/components/flows/common/FlowPanelPlacementPicker.svelte'
	import { overlayStack } from '$lib/components/common/overlayHost.svelte'
	import { FlowService, type FlowModule, type FlowNote, type Job, type OpenFlow } from '../../gen'
	import { findStepPath, parseExpandedSubflowId } from '$lib/components/restartFromStepPath'
	import { expandedSubflowParentId } from '../flows/expandedSubflowStep'
	import { sendUserToast } from '$lib/utils'
	import { AI_OR_ASSET_NODE_TYPES, NODE, type GraphModuleState } from '.'
	import { isTriggerStep } from '$lib/components/flows/flowStepSettings'
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
		type InlineScript,
		type InsertKind,
		type NodeLayout,
		type OnSelectedIteration,
		type SimplifiableFlow
	} from './graphBuilder.svelte'
	import ModuleNode from './renderers/nodes/ModuleNode.svelte'
	import FailureModuleNode from './renderers/nodes/FailureModuleNode.svelte'
	import InputNode from './renderers/nodes/InputNode.svelte'
	import BranchAllStart from './renderers/nodes/BranchAllStart.svelte'
	import BranchAllEndNode from './renderers/nodes/BranchAllEndNode.svelte'
	import ForLoopEndNode from './renderers/nodes/ForLoopEndNode.svelte'
	import ForLoopStartNode from './renderers/nodes/ForLoopStartNode.svelte'
	import ResultNode from './renderers/nodes/ResultNode.svelte'
	import BaseEdge from './renderers/edges/BaseEdge.svelte'
	import EmptyEdge from './renderers/edges/EmptyEdge.svelte'
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
	import {
		linkedAgentToolsForScope,
		linkedAgentToolsVersion,
		linkedToolsScope,
		releaseLinkedToolsScope,
		retainLinkedToolsScope
	} from '$lib/components/flows/linkedAgentToolsStore.svelte'
	import NewAiToolNode from './renderers/nodes/NewAIToolNode.svelte'
	import NoteNode from './renderers/nodes/NoteNode.svelte'
	import CollapsedGroupNode from './renderers/nodes/CollapsedGroupNode.svelte'
	import GroupHeadNode from './renderers/nodes/GroupHeadNode.svelte'
	import GroupEndNode from './renderers/nodes/GroupEndNode.svelte'
	import NoteTool from './NoteTool.svelte'
	import SelectionBoundingBox from './SelectionBoundingBox.svelte'
	import GroupOverlay from './GroupOverlay.svelte'
	import {
		GroupDisplayState,
		getGroupEditorContext,
		groupKey,
		type FlowGroup
	} from './groupEditor.svelte'
	import { buildStructureTree, computeGroupDepths, type FlowStructureNode } from './flowStructure'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { computeGroupModuleIds } from './groupDetectionUtils'
	import { getAllModules } from '../flows/flowExplorer'
	import SelectionTool from './SelectionTool.svelte'
	import PaneContextMenu from './PaneContextMenu.svelte'
	import { SelectionManager, isFlowLevelPanelTarget } from './selectionUtils.svelte'
	import { ChangeTracker } from '$lib/svelte5Utils.svelte'
	import { NoteManager } from './noteManager.svelte'
	import type { MoveManager } from './moveManager.svelte'
	import DragCoordinator from './DragCoordinator.svelte'
	import { jobToGraphModuleState, type ModulesTestStates } from '../modulesTest.svelte'
	import { compoundLayout } from './compoundLayout'
	import { deepEqual } from 'fast-equals'
	import type { AssetWithAltAccessType } from '../assets/lib'
	import { computeNodeExtraSpace } from './nodeExtraSpace'
	import type { ModuleActionInfo } from '$lib/components/flows/flowDiff'
	import { setGraphContext } from './graphContext'
	import { setFlowRunStatusContext } from './flowRunStatus.svelte'
	import { computeNoteNodes } from './noteUtils.svelte'
	import { Tooltip } from '../meltComponents'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import {
		resolveSelectedModuleIds,
		locateModules,
		areContiguousSiblings
	} from '../flows/multiSelectUtils'

	let useDataflow: Writable<boolean | undefined> = writable<boolean | undefined>(false)
	let showAssets: Writable<boolean | undefined> = writable<boolean | undefined>(true)
	let showNotes = $state(true)

	const triggerContext = getContext<TriggerContext>('TriggerContext')
	const overlays = overlayStack()

	// Create diffManager instance for this FlowGraphV2
	const diffManager = createFlowDiffManager()

	let fullWidth = 0
	let width = $state(0)

	let simplifiableFlow: SimplifiableFlow | undefined = $state(undefined)

	interface Props {
		success?: boolean | undefined
		modules?: FlowModule[] | undefined
		groupedModules?: FlowStructureNode[]
		groupError?: unknown
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
		// Flow path for the linked-agent tools bucket. Separate from `path` because that one also
		// drives the Trigger node, which read-only viewers must not render.
		linkedToolsPath?: string | undefined
		newFlow?: boolean
		insertable?: boolean
		earlyStop?: boolean
		cache?: boolean
		scroll?: boolean
		moveManager?: MoveManager
		// Download: display a top level button to open the graph in a new tab
		download?: boolean
		fullSize?: boolean
		disableAi?: boolean
		triggerNode?: boolean
		workspace?: string
		editMode?: boolean
		allowSimplifiedPoll?: boolean
		expandedSubflows?: Record<string, { modules: FlowModule[]; groups?: FlowGroup[] }>
		isOwner?: boolean
		isRunning?: boolean
		individualStepTests?: boolean
		flowJob?: Job | undefined
		showJobStatus?: boolean
		suspendStatus?: Record<string, { job: Job; nb: number }>
		noteMode?: boolean
		notes?: FlowNote[]
		groups?: FlowGroup[]
		groupDisplayState?: GroupDisplayState
		chatInputEnabled?: boolean
		multiSelectEnabled?: boolean
		onDeleteMultiple?: (ids: string[]) => void
		onDuplicateMultiple?: (ids: string[]) => void
		onMoveMultiple?: (ids: string[]) => void
		movingIds?: string[]
		onDelete?: (id: string) => void
		/** Forget the run state of a node that only mirrors a run (the error handler marker), so it
		 * stops being rendered. Must not touch the flow itself. */
		onDismissRunNode?: (id: string) => void
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
			/** Saved `ai_agent` resource the inserted agent step links to, for `kind: 'aiagent'`. */
			agentPath?: string
			kind: InsertKind
			expandGroup?: { groupId: string; position: 'top' | 'bottom' }
		}) => Promise<void>
		onNewBranch?: (id: string) => Promise<void>
		onSelect?: (id: string | FlowModule) => void
		onDeleteBranch?: (detail: { id: string; index: number }) => Promise<void>
		onChangeId?: (detail: { id: string; newId: string; deps: Record<string, string[]> }) => void
		onMove?: (id: string) => void
		onDuplicate?: (id: string) => void
		onUpdateMock?: (detail: { mock: FlowModule['mock']; id: string }) => void
		onTestUpTo?: ((id: string) => void) | undefined
		onSelectedIteration?: OnSelectedIteration
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
		controlsPosition?: 'top' | 'bottom'
		outerDivClass?: string
		/** Fires when the computed graph height changes. Diff views can use
		 * this to equalize heights of side-by-side graphs. */
		onHeight?: (height: number) => void
	}

	let {
		onInsert = undefined,
		onDelete = undefined,
		onDismissRunNode = undefined,
		onMove = undefined,
		onDuplicate = undefined,
		onDeleteBranch = undefined,
		onNewBranch = undefined,
		onSelect = undefined,
		onChangeId = undefined,
		onUpdateMock = undefined,
		onSelectedIteration = undefined,
		success = undefined,
		modules = [],
		groupedModules: groupedModulesProp = undefined,
		groupError = undefined,
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
		linkedToolsPath = undefined,
		newFlow = false,
		insertable = false,
		earlyStop = false,
		cache = false,
		scroll = false,
		moveManager = undefined,
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
		groups = undefined,
		groupDisplayState: groupDisplayStateProp = undefined,
		exitNoteMode = undefined,
		onNotePositionUpdate = undefined,
		chatInputEnabled = false,
		sharedViewport = undefined,
		onViewportChange = undefined,
		leftHeader = undefined,
		diffBeforeFlow = undefined,
		currentInputSchema = undefined,
		markRemovedAsShadowed = false,
		multiSelectEnabled = false,
		onDeleteMultiple = undefined,
		onDuplicateMultiple = undefined,
		onMoveMultiple = undefined,
		movingIds = undefined,
		controlsPosition = 'top',
		outerDivClass = '',
		onHeight = undefined
	}: Props = $props()

	// Hold the scope this graph draws from while it is mounted: the store's cap must never drop a
	// bucket that something on screen is reading, and nothing would refetch it afterwards.
	$effect(() => {
		const scope = linkedToolsScope(workspace, linkedToolsPath ?? path)
		retainLinkedToolsScope(scope)
		return () => releaseLinkedToolsScope(scope)
	})

	// Initialize note manager with fine-grained reactivity
	const noteManager = new NoteManager(
		() => notes ?? [],
		(newNodes) => {
			nodes = newNodes
		},
		() => nodes
	)

	const groupDisplayState =
		untrack(() => groupDisplayStateProp) ?? new GroupDisplayState(() => groups ?? [])

	// Runtime text height tracking for notes (not stored in FlowNote)
	let noteTextHeights = $state<Record<string, number>>({})

	// Reference to pane context menu component
	let paneContextMenu: PaneContextMenu | undefined = $state(undefined)
	let flowContainer: HTMLDivElement | undefined = $state(undefined)

	// Hover tracking for group overlay

	// Selection manager - create one if not provided
	let selectionManager = untrack(() => selectionManagerProp) || new SelectionManager()
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
	const topPadding = untrack(() => editMode) ? 100 : 24
	const yOffset = calculateNoteGap(untrack(() => notes)) + topPadding

	setGraphContext({
		selectionManager: selectionManager,
		useDataflow,
		showAssets,
		noteManager,
		moveManager: untrack(() => moveManager),
		clearFlowSelection,
		yOffset,
		diffManager,
		getFlowNodes: () => currentGraphNodeDeps,
		groupDisplayState
	} as any)

	const flowRunStatus = setFlowRunStatusContext()
	$effect(() => {
		flowRunStatus.flowJob = flowJob
	})
	$effect(() => {
		flowRunStatus.suspendStatus = suspendStatus
	})
	$effect(() => {
		// Each step's state object is replaced rather than mutated, so reading the top level
		// catches every status change. Walking deeper would subscribe to every step's args and
		// result on the hottest path in the graph.
		Object.values(flowModuleStates ?? {})
		// The loader mutates `flow_status` on the job it already handed us, so subscribing to the
		// test state alone would never see an agent's calls land.
		Object.values(testModuleStates?.states ?? {}).forEach((s) => [
			s.loading,
			s.testJob?.['flow_status']?.modules?.[0]?.agent_actions?.length
		])
		untrack(() => {
			// Testing one step is its own small run, and its agent calls arrive on the test job
			// rather than the flow's states. Fold them in so the renderers keep a single source.
			let states = flowModuleStates
			for (const [id, testState] of Object.entries(testModuleStates?.states ?? {})) {
				const tested = jobToGraphModuleState(testState)
				if (!tested?.agent_actions) continue
				states = { ...(states ?? {}), [id]: { ...(states?.[id] ?? {}), ...tested } }
			}
			flowRunStatus.setModuleStates(states)
		})
	})

	if (triggerContext && untrack(() => allowSimplifiedPoll)) {
		if (isSimplifiable(untrack(() => modules))) {
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
		data?: { assets?: AssetWithAltAccessType[]; module?: any }
	}
	type NodePos = { position: { x: number; y: number } }
	let lastNodes:
		| [NodeDep[], Map<string, { top: number; bottom: number }> | undefined, (NodeDep & NodePos)[]]
		| undefined = undefined
	let currentGraphNodeDeps: { id: string; parentIds?: string[] }[] = $state([])

	// Keep canCreateGroup in sync for consumers (SelectionBoundingBox, FlowSelectionPanel, etc.)
	const groupEditorCtx = getGroupEditorContext()

	$effect(() => {
		if (!groupEditorCtx) return
		const ids = selectionManager.selectedIds
		groupEditorCtx.canCreateGroup.val =
			ids.length >= 1 && groupEditorCtx.groupEditor.canCreateGroup(ids, currentGraphNodeDeps)
	})

	let lastGroupDimensions: Map<string, { width: number; height: number }> | undefined = undefined

	function layoutNodes(
		nodes: NodeDep[],
		nodeExtraSpace?: Map<string, { top: number; bottom: number; left: number; right: number }>
	): (NodeDep & NodePos)[] {
		let lastResult = lastNodes?.[2]
		if (
			lastResult &&
			deepEqual(nodes, lastNodes?.[0]) &&
			deepEqual(nodeExtraSpace, lastNodes?.[1])
		) {
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

		// Run recursive compound layout with pre-computed extra space
		const layoutResult = compoundLayout(
			nodes,
			{
				nodeWidth: NODE.width,
				nodeHeight: NODE.height,
				gapH: NODE.gap.horizontal,
				gapV: NODE.gap.vertical
			},
			nodeExtraSpace
		)
		const { positions, bbox } = layoutResult
		lastGroupDimensions = layoutResult.groupDimensions

		const xCenter = (fullSize ? fullWidth : width) / 2 - bbox.width / 2 - (width - fullWidth) / 2

		// Center horizontally
		const newNodes = nodes.map((n) => ({
			id: n.id,
			position: {
				x: (positions.get(n.id)?.x ?? 0) + xCenter - NODE.width / 2,
				y: positions.get(n.id)?.y ?? 0
			}
		}))

		lastNodes = [nodes, nodeExtraSpace, newNodes]
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
		select: (modId, opts) => {
			// AI tools are not selectable by the flow. Selection has to be refactored to be simplier.
			// Flow-level panels reach selection only through here, so they must go through
			// selectId or their intent (and the modal panel) never fires.
			if (
				nodes.find((n) => n.data?.moduleId === modId)?.type === 'aiTool' ||
				isFlowLevelPanelTarget(modId)
			) {
				selectionManager.selectId(modId, opts)
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
		duplicate: (detail) => {
			onDuplicate?.(detail.id)
		},
		selectedIteration: (detail) => {
			onSelectedIteration?.(detail)
		},
		simplifyFlow: (detail) => {
			triggerContext?.simplifiedPoll.set(detail)
		},
		expandSubflow: async (id: string, path: string) => {
			// Reads the subflow's *current* definition, which a share link deliberately does
			// not cover: it authorizes the run's job subtree, not the workspace's flow
			// library. So a share-link viewer (and any anonymous one) is refused here — name
			// that case, but only when the error actually says so.
			let flow: OpenFlow
			try {
				flow = await FlowService.getFlowByPath({ workspace: workspace, path })
			} catch (err) {
				const denied = err?.status === 401 || err?.status === 403
				sendUserToast(
					`Could not expand subflow ${path}: ${
						denied
							? "viewing a subflow's definition requires being logged in with access to it"
							: (err?.body ?? err)
					}`,
					true
				)
				return
			}
			expandedSubflows[id] = { modules: flow.value.modules, groups: flow.value.groups }
			expandedSubflows = expandedSubflows
		},
		minimizeSubflow: (id: string) => {
			delete expandedSubflows[id]
			expandedSubflows = expandedSubflows
		},
		expandGroup: (groupId: string) => {
			groupDisplayState.expandGroup(groupId)
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
		},
		dismissRunNode: (id: string) => {
			onDismissRunNode?.(id)
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

	// Derived state for multi-select operations
	let resolvedModuleIds = $derived(
		resolveSelectedModuleIds(selectionManager.selectedIds, effectiveModules ?? [])
	)
	let canMoveSelected = $derived(
		resolvedModuleIds.length > 0 &&
			areContiguousSiblings(locateModules(resolvedModuleIds, effectiveModules ?? []))
	)

	// Initialize moduleTracker with effectiveModules
	let moduleTracker = $state(new ChangeTracker<FlowModule[]>([]))

	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])

	let height = $state(0)

	/**
	 * A run only changes what the steps display, never the shape of the graph, but every
	 * status poll rebuilds these arrays from scratch. xyflow re-measures every node it is
	 * handed a new object for, and Svelte destroys and re-creates every edge, so handing
	 * back fresh identities re-renders a graph that did not change. Reuse the previous
	 * object wherever it still deep-equals the new one.
	 */
	function reuseUnchanged<T extends { id: string }>(previous: T[], next: T[]): T[] {
		const previousById = new Map(previous.map((item) => [item.id, item]))
		let changed = previous.length !== next.length
		const reconciled = next.map((item, index) => {
			const before = previousById.get(item.id)
			if (before && deepEqual(before, item)) {
				changed ||= previous[index] !== before
				return before
			}
			changed = true
			return item
		})
		return changed ? reconciled : previous
	}

	// Keyed by the source node, so a node that survived reconciliation keeps its offset
	// object too — remapping every node would undo the identity reuse above.
	let offsetNodeCache = new WeakMap<Node, Node>()
	let offsetCacheKey: string | undefined = undefined

	// Derived nodes with yOffset applied to all nodes uniformly and selectable flag set to false if notSelectable is true
	const nodesWithOffset = $derived.by(() => {
		const cacheKey = `${yOffset}:${notSelectable}`
		if (cacheKey !== offsetCacheKey) {
			offsetNodeCache = new WeakMap<Node, Node>()
			offsetCacheKey = cacheKey
		}
		return nodes.map((node) => {
			const cached = offsetNodeCache.get(node)
			if (cached) {
				return cached
			}
			const mapped =
				node.type && !AI_OR_ASSET_NODE_TYPES.includes(node.type)
					? {
							...node,
							position: { ...node.position, y: node.position.y + yOffset },
							selectable: notSelectable ? false : node.selectable
						}
					: {
							...node,
							selectable: notSelectable ? false : node.selectable
						}
			offsetNodeCache.set(node, mapped)
			return mapped
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
		// xyflow owns `selected` on the objects it was handed, and drops it only when it sees a
		// node it does not recognise. Serving the cached mapping back would hand it the very
		// object it marked selected, so the clear has to go through fresh objects.
		offsetNodeCache = new WeakMap<Node, Node>()
		nodes = nodes.map((node) => {
			if (node.selected) {
				return { ...node, selected: false }
			}
			return node
		})
	}

	// Keyboard event handling
	function handleKeyDown(event: KeyboardEvent) {
		// Escape belongs to the topmost overlay. This listener is on `document` and theirs
		// are on `window`, so this one always runs first — without the guard, dismissing a
		// modal or picker would also clear the selection under it and lose the user's step.
		if (event.key === 'Escape' && overlays.val.length > 0) {
			return
		}
		selectionManager.handleKeyDown(event)
		noteManager.handleKeyDown(event)
		if (event.key === 'Escape') {
			if (noteMode) {
				exitNoteMode?.()
			}
		}
		if ((event.key === 'Backspace' || event.key === 'Delete') && editMode) {
			const active = document.activeElement
			if (active && active !== document.body && !flowContainer?.contains(active)) {
				return
			}
			if (
				active instanceof HTMLInputElement ||
				active instanceof HTMLTextAreaElement ||
				active?.getAttribute('contenteditable') === 'true'
			) {
				return
			}
			if (noteManager.selectedNoteId && noteEditorContext) {
				noteEditorContext.noteEditor.deleteNote(noteManager.selectedNoteId)
				noteManager.clearNoteSelection()
				return
			}
			if (resolvedModuleIds.length > 1) {
				onDeleteMultiple?.(resolvedModuleIds)
			} else if (resolvedModuleIds.length === 1) {
				onDelete?.(resolvedModuleIds[0])
			} else if (selectedId) {
				onDelete?.(selectedId)
			}
		}
	}

	async function updateStores() {
		if (graph.error) {
			return
		}

		const graphNodeDeps = Object.values(graph.nodes).map((n) => ({
			id: n.id,
			parentIds: n.parentIds,
			data: { assets: (n.data as any).assets, module: (n.data as any).module }
		}))
		currentGraphNodeDeps = graphNodeDeps

		// Pre-compute extra space per node for assets, AI tools, group notes, group headers
		const resolvedLinkedTools = linkedAgentToolsForScope(
			linkedToolsScope(workspace, linkedToolsPath ?? path)
		)
		const nodeExtraSpace = computeNodeExtraSpace(graphNodeDeps, {
			showAssets: $showAssets ?? true,
			showNotes,
			notes,
			noteTextHeights,
			groupDisplayState,
			insertable,
			flowModuleStates,
			linkedAgentTools: resolvedLinkedTools
		})

		// Layout with extra space baked into sugiyama
		let layoutedNodes = layoutNodes(graphNodeDeps, nodeExtraSpace)
		let newNodes: (Node & NodeLayout)[] = layoutedNodes.map((n) => {
			const merged = { ...n, ...graph.nodes[n.id] }
			// Augment group head nodes with wrapper dimensions from compound layout
			if (graph.nodes[n.id]?.type === 'groupHead' && lastGroupDimensions?.has(n.id)) {
				const dims = lastGroupDimensions.get(n.id)!
				merged.data = { ...merged.data, wrapperWidth: dims.width, wrapperHeight: dims.height }
			}
			return merged
		})

		// Compute asset visual nodes (no position remapping)
		let assetNodesResult = $showAssets
			? computeAssetNodes(
					newNodes.map((n) => ({
						data: { assets: n.data?.assets as AssetWithAltAccessType[] },
						id: n.id,
						position: n.position
					}))
				)
			: undefined

		// Compute AI tool visual nodes (no position remapping)
		let aiToolNodesResult = computeAIToolNodes(
			newNodes,
			eventHandler,
			insertable,
			flowModuleStates,
			resolvedLinkedTools
		)

		let finalNodes: (Node & NodeLayout)[] = [
			...newNodes,
			...(assetNodesResult?.newAssetNodes ?? []),
			...aiToolNodesResult.toolNodes
		]

		// Collect module IDs hidden inside collapsed groups so note cleanup preserves them
		const collapsedModuleIds = new Set<string>()
		for (const n of finalNodes) {
			if (n.type === 'collapsedGroup') {
				const modules = (n.data as any)?.modules as FlowModule[] | undefined
				if (modules) {
					for (const m of modules) {
						collapsedModuleIds.add(m.id)
					}
				}
			}
		}

		// Compute note nodes (no position remapping)
		let noteNodesResult = showNotes
			? computeNoteNodes(
					finalNodes.map((n) => ({
						id: n.id,
						position: n.position,
						parentIds: n.parentIds,
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
					noteEditorContext,
					collapsedModuleIds.size > 0 ? collapsedModuleIds : undefined
				)
			: undefined

		// update nodes
		nodes = reuseUnchanged(nodes, [...finalNodes, ...(noteNodesResult?.noteNodes ?? [])])

		edges = reuseUnchanged(edges, [
			...(assetNodesResult?.newAssetEdges ?? []),
			...aiToolNodesResult.toolEdges,
			...graph.edges
		])

		await tick()
		updateHeight()
	}

	function updateHeight() {
		if (nodes.length === 0) {
			height = minHeight
		} else {
			const minY = Math.min(...nodes.map((n) => n.position.y))
			const maxBottom = Math.max(...nodes.map((n) => n.position.y + NODE.height + 100))
			const computed = maxBottom - minY
			height = Math.max(Math.min(computed, maxHeight ?? computed), minHeight)
		}
		onHeight?.(height)
	}

	$effect(() => {
		// Track both bounds — updateHeight() reads both, so missing one (as
		// maxHeight was) leaves height stale when only that bound changes.
		minHeight
		maxHeight
		untrack(() => updateHeight())
	})

	const nodeTypes = {
		input2: InputNode,
		module: ModuleNode,
		failureModule: FailureModuleNode,
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
		note: NoteNode,
		collapsedGroup: CollapsedGroupNode,
		groupHead: GroupHeadNode,
		groupEnd: GroupEndNode
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
		currentGroups
		// The poll replaces `flowJob` on every tick and is what makes the untracked
		// `flowModuleStates` above get re-read, so it stays a dependency. It is deliberately
		// not handed to graphBuilder: anything put there lands in every node and edge's data,
		// and a per-tick value there re-creates the whole graph. Renderers read it from
		// FlowRunStatus instead.
		flowJob
		suspendStatus

		const collapsedGroupIds = new Set(
			allGroups
				.filter((g) => groupDisplayState.isRuntimeCollapsed(groupKey(g)))
				.map((g) => groupKey(g))
		)

		if (groupError) {
			return { nodes: {}, edges: [], error: groupError }
		}

		// Use provided structure tree (from proxy) or build locally (diff mode / read-only)
		let gm: FlowStructureNode[] | undefined = groupedModulesProp
		if (!gm) {
			const allGroups = groups ?? []
			const graphGroups = allGroups.map((g) => ({
				...g,
				id: groupKey(g),
				moduleIds: untrack(() =>
					computeGroupModuleIds(g.start_id, g.end_id, getAllModules(effectiveModules ?? []))
				)
			}))
			try {
				gm = buildStructureTree(
					stateSnapshot(untrack(() => effectiveModules) ?? []) as FlowModule[],
					graphGroups
				)
			} catch (e) {
				return { nodes: {}, edges: [], error: e }
			}
		}

		const result = graphBuilder(
			gm,
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
				showJobStatus,
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
			simplifiableFlow,
			triggerNode ? path : undefined,
			expandedSubflows,
			showNotes,
			collapsedGroupIds
		)
		return { ...result, structureTree: gm }
	})
	let hideAssetsToggle = $derived(
		$showAssets && Object.values(nodes).every((n) => n.type !== 'asset')
	)
	let hideNotesToggle = $derived(
		(!notes || notes.length === 0) && !(groups ?? []).some((g) => g.note != null)
	)

	let currentGroupDepths = $derived(
		'structureTree' in graph && graph.structureTree ? computeGroupDepths(graph.structureTree) : {}
	)

	// All groups including those from expanded subflows (for overlay rendering)
	let allGroups = $derived.by(() => {
		const base = groups ?? []
		const subflowGroups = Object.values(expandedSubflows).flatMap((sf) => sf.groups ?? [])
		return subflowGroups.length > 0 ? [...base, ...subflowGroups] : base
	})

	// Track groups for re-layout when groups change
	let currentGroups = $derived(groups ?? [])

	/**
	 * An agent's tool calls become nodes, and they land one at a time while the step runs.
	 * Nothing else the graph reads changes as they arrive — the run only appends to an
	 * existing step's state — so without this the tools all appear at once when the step ends.
	 */
	let agentActionsVersion = $derived.by(() => {
		// The editor draws the agent's declared tools and ignores the run's calls, so neither the
		// layout nor the tool nodes can change as they land.
		if (insertable) return ''
		let version = ''
		for (const [id, state] of Object.entries(flowModuleStates ?? {})) {
			const actions = state?.agent_actions
			if (!actions) continue
			version += `${id}:${actions.length}:`
			for (const action of actions) {
				version += `${action.type}/${(action as { function_name?: string }).function_name ?? ''},`
			}
		}
		return version
	})

	$effect(() => {
		;[
			graph,
			allowSimplifiedPoll,
			$showAssets,
			showNotes,
			noteManager.renderCount,
			currentGroups,
			groupDisplayState.renderCount,
			agentActionsVersion,
			// A linked step's tools resolve asynchronously; recompute tool nodes when they land.
			linkedAgentToolsVersion()
		]
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

	let latestReload = 0

	/** Flow an expanded subflow node stands for, read from the step it inlines: the edited
	 * flow for a top-level expansion, the enclosing expansion's modules otherwise. */
	function expandedSubflowPath(nodeId: string): string | undefined {
		const parentId = expandedSubflowParentId(nodeId)
		const parentModules = parentId == undefined ? modules : expandedSubflows[parentId]?.modules
		const stepId = parseExpandedSubflowId(nodeId)?.leaf ?? nodeId
		const value = parentModules && findStepPath(parentModules, stepId)?.target.value
		return value && value.type === 'flow' ? value.path : undefined
	}

	/** Refetch the steps inlined by every expanded subflow, e.g. after one was deployed from
	 * the flow editor drawer. Outermost first, so a nested expansion resolves its path from
	 * its refreshed parent: a step now pointing at another flow must not keep rendering the
	 * one it pointed at when it was expanded. */
	export async function reloadExpandedSubflows() {
		const reload = ++latestReload
		const ids = Object.keys(expandedSubflows).sort(
			(a, b) =>
				(parseExpandedSubflowId(a)?.subflowSteps.length ?? 0) -
				(parseExpandedSubflowId(b)?.subflowSteps.length ?? 0)
		)
		for (const id of ids) {
			// A later reload owns the state from here on.
			if (reload !== latestReload) return
			const expansion = expandedSubflows[id]
			if (expansion == undefined) continue
			const path = expandedSubflowPath(id)
			if (path == undefined) {
				delete expandedSubflows[id]
				continue
			}
			try {
				const flow = await FlowService.getFlowByPath({ workspace: workspace, path })
				if (reload !== latestReload) return
				// While this request was in flight the user may have collapsed the expansion, or
				// collapsed it and re-expanded onto another flow: only commit onto the very
				// expansion this response was fetched for.
				if (expandedSubflows[id] !== expansion) continue
				expandedSubflows[id] = { modules: flow.value.modules, groups: flow.value.groups }
			} catch (err) {
				sendUserToast(`Could not reload expanded subflow ${path}: ${err.body ?? err}`, true)
			}
		}
		expandedSubflows = expandedSubflows
	}

	export function createGroupFromSelection(ids: string[]) {
		if (groupEditorCtx?.groupEditor) {
			groupEditorCtx.groupEditor.createGroup(ids, currentGraphNodeDeps)
			tick().then(() => {
				clearFlowSelection()
				selectionManager.clearSelection()
			})
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
	class="overflow-clip relative {outerDivClass}"
	bind:clientWidth={debouncedWidth}
	bind:this={flowContainer}
>
	{#if graph?.error}
		<div class="center-center p-2 mt-20">
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
			{#if moveManager}
				<DragCoordinator
					{moveManager}
					eventHandlers={eventHandler}
					{edges}
					nodes={nodesWithOffset}
				/>
			{/if}
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
				deleteKey={null}
				nodesDraggable={false}
				--background-color={false}
			>
				<div class="absolute inset-0 !bg-surface-secondary h-full" id="flow-graph-v2"></div>

				{#if noteMode}
					<NoteTool {exitNoteMode} {yOffset} />
				{/if}

				{#if multiSelectEnabled}
					<SelectionBoundingBox
						selectedNodes={selectionManager.selectedIds.filter((id) =>
							nodesWithOffset.some((n) => n.id === id)
						)}
						allNodes={nodesWithOffset as (Node & { type: string })[]}
						onDeleteSelected={() => onDeleteMultiple?.(resolvedModuleIds)}
						onDuplicateSelected={() => onDuplicateMultiple?.(resolvedModuleIds)}
						onMoveSelected={() => onMoveMultiple?.(resolvedModuleIds)}
						onCancelMove={() => onMoveMultiple?.(movingIds ?? [])}
						{canMoveSelected}
						isMoving={movingIds != null && movingIds.length > 0}
						{resolvedModuleIds}
					/>
				{/if}

				<GroupOverlay
					allNodes={nodesWithOffset as (Node & { type: string })[]}
					groups={allGroups}
					groupDepths={currentGroupDepths}
				/>

				<!-- SelectionTool for handling selection changes and filtering -->
				<SelectionTool {selectionManager} clearGraphSelection={clearFlowSelection} />

				{#if leftHeader}
					<div class="absolute top-2 left-2 z-10">
						{@render leftHeader()}
					</div>
				{:else}
					<!-- Their built-in glyphs are fill-based and sized differently from every other
					     icon in the editor, so the bar is built from lucide throughout. -->
					<Controls
						class="wm-flow-controls"
						position={controlsPosition === 'bottom' ? 'bottom-right' : 'top-right'}
						orientation="horizontal"
						showZoom={false}
						showFitView={false}
						showLock={false}
					>
						<GraphZoomControls fitViewNodes={nodes.filter((n) => n.type !== 'note')} />
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
											encodeState({ modules, failureModule, preprocessorModule, notes, groups })
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
						<FlowPanelPlacementPicker variant="control" placement="top-end" />
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

	/* xy-flow's own rules are nested, so `.svelte-flow__controls-button svg` and the like
	   match ours exactly and load order decides. Scoping by the class we pass to <Controls>
	   outranks them instead of racing them. */
	:global(.svelte-flow__controls.wm-flow-controls) {
		/* Name the colour rather than leaning on the base layer's bare `.border`: preflight
		   sets a light `border-color` on every element, so whenever that base rule does not
		   make it into the bundle alongside this one the bar draws a white outline. */
		@apply overflow-hidden rounded-md border border-border-light;
		box-shadow: none;
	}
	:global(.wm-flow-controls .svelte-flow__controls-button) {
		@apply bg-surface text-primary;
		width: 32px;
		height: 30px;
		padding: 8px;
	}
	:global(.wm-flow-controls .svelte-flow__controls-button:hover) {
		@apply bg-surface-hover;
	}
	:global(.wm-flow-controls.horizontal .svelte-flow__controls-button) {
		@apply border-r border-gray-200 dark:border-gray-700;
	}
	:global(.wm-flow-controls.horizontal .svelte-flow__controls-button:last-child) {
		@apply border-r-0;
	}
	/* Every glyph in this bar is lucide, so undo their base `fill: currentColor` — it beats
	   lucide's inline fill="none" and would flood the stroke icons solid — and lift the
	   12px cap that keeps them off the editor's icon scale. */
	:global(.wm-flow-controls .svelte-flow__controls-button svg) {
		max-width: 16px;
		max-height: 16px;
		fill: none;
		stroke: currentColor;
	}

	:global(.svelte-flow__edgelabel-renderer) {
		@apply z-50;
	}

	:global(.svelte-flow__selection) {
		display: none;
		pointer-events: none;
	}

	:global(.svelte-flow__selection-wrapper) {
		pointer-events: none !important;
	}
</style>
