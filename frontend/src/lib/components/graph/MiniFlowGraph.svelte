<script lang="ts">
	import { writable } from 'svelte/store'
	import { SvelteFlow, SvelteFlowProvider, type Node, type Edge, type Viewport } from '@xyflow/svelte'
	import { setGraphContext } from './graphContext'
	import { SelectionManager } from './selectionUtils.svelte'
	import { createFlowDiffManager } from '../flows/flowDiffManager.svelte'

	import ModuleNode from './renderers/nodes/ModuleNode.svelte'
	import InputNode from './renderers/nodes/InputNode.svelte'
	import BranchAllStart from './renderers/nodes/BranchAllStart.svelte'
	import BranchAllEndNode from './renderers/nodes/BranchAllEndNode.svelte'
	import ForLoopEndNode from './renderers/nodes/ForLoopEndNode.svelte'
	import ForLoopStartNode from './renderers/nodes/ForLoopStartNode.svelte'
	import ResultNode from './renderers/nodes/ResultNode.svelte'
	import BranchOneStart from './renderers/nodes/BranchOneStart.svelte'
	import BranchOneEndNode from './renderers/nodes/branchOneEndNode.svelte'
	import SubflowBound from './renderers/nodes/SubflowBound.svelte'
	import NoBranchNode from './renderers/nodes/NoBranchNode.svelte'
	import AssetNode from './renderers/nodes/AssetNode.svelte'
	import AssetsOverflowedNode from './renderers/nodes/AssetsOverflowedNode.svelte'
	import AiToolNode from './renderers/nodes/AIToolNode.svelte'
	import NewAiToolNode from './renderers/nodes/NewAIToolNode.svelte'
	import BaseEdge from './renderers/edges/BaseEdge.svelte'
	import EmptyEdge from './renderers/edges/EmptyEdge.svelte'
	import DataflowEdge from './renderers/edges/DataflowEdge.svelte'
	import HiddenBaseEdge from './renderers/edges/HiddenBaseEdge.svelte'

	let {
		nodes,
		edges,
		width,
		height,
		initialViewport
	}: { nodes: Node[]; edges: Edge[]; width: number; height: number; initialViewport?: Viewport } =
		$props()

	setGraphContext({
		selectionManager: new SelectionManager(),
		useDataflow: writable(false),
		showAssets: writable(false),
		freeDrag: writable(false),
		diffManager: createFlowDiffManager()
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
	const fitViewOptions = { padding: 0 }
</script>

<div style="width: {width}px; height: {height}px;">
	<SvelteFlowProvider>
		<SvelteFlow
			{nodes}
			{edges}
			{nodeTypes}
			{edgeTypes}
			fitView={!initialViewport}
			{fitViewOptions}
			{initialViewport}
			nodesDraggable={false}
			elementsSelectable={false}
			panOnDrag={false}
			zoomOnScroll={false}
			zoomOnPinch={false}
			zoomOnDoubleClick={false}
			preventScrolling={false}
			{proOptions}
			style="width: {width}px; height: {height}px; background: transparent;"
		/>
	</SvelteFlowProvider>
</div>
