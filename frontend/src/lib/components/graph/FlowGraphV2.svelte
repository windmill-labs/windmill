<script lang="ts">
	import { type FlowModule } from '../../gen'
	import { NODE, type GraphModuleState } from '.'
	import { setContext } from 'svelte'

	import { writable, type Writable } from 'svelte/store'
	import '@xyflow/svelte/dist/style.css'
	import type { FlowInput } from '../flows/types'
	import {
		SvelteFlow,
		Background,
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
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	let width = 0
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

	let fullWidth = 0
	function layoutNodes(nodes: Node[], edges: Edge[]): { nodes: Node[]; edges: Edge[] } {
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}

		const flattenParentIds = nodes.map((n) => ({
			...n,
			parentIds: n.data?.parentIds ?? []
		}))

		const stratify = dagStratify().id(({ id }: Node) => id)
		const dag = stratify(flattenParentIds)

		let boxSize: any
		try {
			const layout = sugiyama()
				.decross(decrossOpt())
				.coord(coordCenter())
				.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
			boxSize = layout(dag)
		} catch {
			const layout = sugiyama()
				.coord(coordCenter())
				.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
			boxSize = layout(dag)
		}

		const newNodes = dag.descendants().map((des) => ({
			...des.data,
			id: des.data.id,
			position: {
				x: des.x
					? (des.data.data.offset ?? 0) +
					  des.x +
					  (fullSize ? fullWidth : width) / 2 -
					  boxSize.width / 2 -
					  NODE.width / 2
					: 0,
				y: des.y || 0
			}
		}))

		return {
			nodes: newNodes,
			edges
		}
	}

	const { nodes: initialNodes, edges: initialEdges } = graphBuilder(modules)
	const { nodes: layoutedNodes, edges: layoutedEdges } = layoutNodes(initialNodes, initialEdges)

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
	bind:clientWidth={width}
	class="h-full"
>
	<SvelteFlow
		{nodes}
		{edges}
		{edgeTypes}
		{nodeTypes}
		connectionLineType={ConnectionLineType.SmoothStep}
		defaultEdgeOptions={{ type: 'smoothstep' }}
		fitView
		{proOptions}
	>
		<Background class="!bg-surface-secondary" />
		<MiniMap nodeStrokeWidth={3} darkMode />
	</SvelteFlow>
</div>

<style>
	:global(.svelte-flow__handle) {
		opacity: 0;
	}
</style>
