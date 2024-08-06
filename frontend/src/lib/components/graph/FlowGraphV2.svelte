<script lang="ts">
	import { type FlowModule } from '../../gen'
	import { type GraphModuleState } from '.'
	import { setContext } from 'svelte'

	import { writable, type Writable } from 'svelte/store'
	import '@xyflow/svelte/dist/style.css'
	import type { FlowInput } from '../flows/types'
	import {
		SvelteFlow,
		Background,
		Position,
		type Node,
		type Edge,
		ConnectionLineType,
		MiniMap
		// @ts-ignore
	} from '@xyflow/svelte'
	// @ts-ignore
	import dagre from '@dagrejs/dagre'
	import graphBuilder from './graphBuilder'
	import ModuleNode from './renderers/nodes/ModuleNode.svelte'
	import InputNode from './renderers/nodes/InputNode.svelte'
	import BranchAllStart from './renderers/nodes/BranchAllStart.svelte'
	import BranchAllEndNode from './renderers/nodes/branchAllEndNode.svelte'
	import ForLoopEndNode from './renderers/nodes/ForLoopEndNode.svelte'
	import ForLoopStartNode from './renderers/nodes/ForLoopStartNode.svelte'
	import ResultNode from './renderers/nodes/ResultNode.svelte'
	import BaseEdge from './renderers/edges/BaseEdge.svelte'
	import EmptyEdge from './renderers/edges/EmptyEdge.svelte'

	export let success: boolean | undefined = undefined
	export let modules: FlowModule[] | undefined = []
	export let failureModule: FlowModule | undefined = undefined
	export let minHeight: number = 0
	export let maxHeight: number | undefined = undefined
	export let notSelectable = false
	export let flowModuleStates: Record<string, GraphModuleState> | undefined = undefined
	export let rebuildOnChange: any = undefined

	export let selectedId: Writable<string | undefined> = writable<string | undefined>(undefined)

	export let insertable = false
	export let moving: string | undefined = undefined
	export let scroll = false
	export let download = false
	export let fullSize = false
	export let disableAi = false
	export let flowInputsStore: Writable<FlowInput | undefined> = writable<FlowInput | undefined>(
		undefined
	)

	setContext<{
		selectedId: Writable<string | undefined>
		flowInputsStore: Writable<FlowInput | undefined>
	}>('FlowGraphContext', { selectedId, flowInputsStore })

	const nodeWidth = 300
	const nodeHeight = 36
	const dagreGraph = new dagre.graphlib.Graph()

	dagreGraph.setDefaultEdgeLabel(() => ({}))

	function getLayoutedElements(nodes: Node[], edges: Edge[], direction = 'TB') {
		dagreGraph.setGraph({ rankdir: direction })

		nodes.forEach((node) => {
			dagreGraph.setNode(node.id, { width: nodeWidth, height: nodeHeight })
		})

		edges.forEach((edge) => {
			dagreGraph.setEdge(edge.source, edge.target, {
				minlen: 1
			})
		})

		dagre.layout(dagreGraph)

		nodes.forEach((node) => {
			const nodeWithPosition = dagreGraph.node(node.id)
			node.targetPosition = Position.Top
			node.sourcePosition = Position.Bottom

			node.position = {
				x: nodeWithPosition.x - nodeWidth / 2,
				y: nodeWithPosition.y - nodeHeight / 2
			}
		})

		return { nodes, edges }
	}

	const { nodes: initialNodes, edges: initialEdges } = graphBuilder(modules)

	const { nodes: layoutedNodes, edges: layoutedEdges } = getLayoutedElements(
		initialNodes,
		initialEdges
	)

	const nodes = writable<Node[]>(layoutedNodes)
	const edges = writable<Edge[]>(layoutedEdges)

	const nodeTypes = {
		input2: InputNode,
		module: ModuleNode,
		branchAllStart: BranchAllStart,
		branchAllEnd: BranchAllEndNode,
		forLoopEnd: ForLoopEndNode,
		forLoopStart: ForLoopStartNode,
		result: ResultNode,
		whileLoopStart: ForLoopStartNode,
		whileLoopEnd: ForLoopEndNode
	}
	const edgeTypes = {
		edge: BaseEdge,
		empty: EmptyEdge
	}

	const proOptions = { hideAttribution: true }
</script>

<div
	style={`min-height: ${minHeight}px; max-height: ${
		maxHeight ? maxHeight + 'px' : 'none'
	}; height:100%;`}
>
	<SvelteFlow
		{nodes}
		{edges}
		{edgeTypes}
		{nodeTypes}
		nodesDraggable={false}
		connectionLineType={ConnectionLineType.SmoothStep}
		defaultEdgeOptions={{ type: 'smoothstep' }}
		fitView
		{proOptions}
	>
		<Background class="!bg-surface-secondary" />
		<MiniMap nodeStrokeWidth={3} />
	</SvelteFlow>
</div>
