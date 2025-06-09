<script lang="ts">
	import { FlowService, type FlowModule } from '../../gen'
	import { NODE, type GraphModuleState } from '.'
	import { getContext, onDestroy, setContext, tick, untrack } from 'svelte'

	import { get, writable, type Writable } from 'svelte/store'
	import '@xyflow/svelte/dist/base.css'
	import type { FlowInput } from '../flows/types'
	import {
		SvelteFlow,
		type Node,
		type Edge,
		ConnectionLineType,
		Controls,
		ControlButton,
		SvelteFlowProvider
	} from '@xyflow/svelte'
	import {
		graphBuilder,
		isTriggerStep,
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
	import { encodeState } from '$lib/utils'
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
	import { deepEqual } from 'fast-equals'
	import ViewportResizer from './ViewportResizer.svelte'

	let useDataflow: Writable<boolean | undefined> = writable<boolean | undefined>(false)

	const triggerContext = getContext<TriggerContext>('TriggerContext')

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
		flowInputsStore?: Writable<FlowInput | undefined>
		triggerNode?: boolean
		workspace?: string
		editMode?: boolean
		allowSimplifiedPoll?: boolean
		expandedSubflows?: Record<string, FlowModule[]>
		onDelete?: (id: string) => void
		onInsert?: (detail: {
			sourceId?: string
			targetId?: string
			branch?: { rootId: string; branch: number }
			index: number
			detail: string
			isPreprocessor?: boolean
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
		onSelectedIteration?: onSelectedIteration
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
		flowInputsStore = writable<FlowInput | undefined>(undefined),
		triggerNode = false,
		workspace = $workspaceStore ?? 'NO_WORKSPACE',
		editMode = false,
		allowSimplifiedPoll = true,
		expandedSubflows = $bindable({})
	}: Props = $props()

	setContext<{
		selectedId: Writable<string | undefined>
		flowInputsStore: Writable<FlowInput | undefined>
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext', { selectedId, flowInputsStore, useDataflow })

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

	let lastNodes: [NodeLayout[], Node[]] | undefined = undefined
	function layoutNodes(nodes: NodeLayout[]): Node[] {
		let lastResult = lastNodes?.[1]
		if (lastResult && deepEqual(nodes, lastNodes?.[0])) {
			return lastResult
		}
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}

		let nodeWidths: Record<string, number> = {}
		const nodes2 = nodes.map((n) => {
			return { ...n, position: { x: 0, y: 0 } }
		})
		for (const n of nodes.reverse()) {
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

		const dag = dagStratify().id(({ id }: Node) => id)(nodes2)

		let boxSize: any
		try {
			console.log(nodeWidths)
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
			...des.data,
			id: des.data.id,
			position: {
				x: des.x
					? // @ts-ignore
						(des.data.data.offset ?? 0) +
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
		}
	}

	let lastModules = structuredClone($state.snapshot(modules))
	let moduleCounter = $state(0)
	function onModulesChange2(modules) {
		if (!deepEqual(modules, lastModules)) {
			console.log('modules changed', modules)
			lastModules = structuredClone($state.snapshot(modules))
			moduleCounter++
		}
	}

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
		let newGraph = graph

		nodes = layoutNodes(newGraph.nodes)
		edges = newGraph.edges
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
		trigger: TriggersNode
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

	$effect(() => {
		allowSimplifiedPoll && onModulesChange(modules ?? [])
	})
	$effect(() => {
		modules && onModulesChange2(modules)
	})
	let graph = $derived.by(() => {
		console.log('graph', moduleCounter)
		moduleCounter
		return graphBuilder(
			untrack(() => modules),
			{
				disableAi,
				insertable,
				flowModuleStates,
				selectedId: $selectedId,
				path,
				newFlow,
				cache,
				earlyStop,
				editMode
			},
			failureModule,
			preprocessorModule,
			eventHandler,
			success,
			$useDataflow,
			$selectedId,
			moving,
			simplifiableFlow,
			triggerNode ? path : undefined,
			expandedSubflows
		)
	})
	$effect(() => {
		;(graph || allowSimplifiedPoll) && updateStores()
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
</script>

{#if insertable}
	<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
{/if}

<div style={`height: ${height}px; max-height: ${maxHeight}px;`} bind:clientWidth={width}>
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
			<ViewportResizer {width} />
			<SvelteFlow
				onpaneclick={(e) => {
					document.dispatchEvent(new Event('focus'))
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
				<div class="absolute inset-0 !bg-surface-secondary"></div>
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
					orientation="horizontal"
					showLock={false}
					showZoom={false}
					showFitView={false}
					class="!shadow-none"
				>
					{#if showDataflow}
						<Toggle
							value={$useDataflow}
							on:change={() => {
								$useDataflow = !$useDataflow
							}}
							size="xs"
							options={{
								right: 'Dataflow'
							}}
						/>
					{/if}
				</Controls>
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
