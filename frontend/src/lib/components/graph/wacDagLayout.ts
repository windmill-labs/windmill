import type { Node, Edge } from '@xyflow/svelte'
import type { WacWorkflowDag, WacDagNode } from '$lib/infer'
import { NODE } from './util'

const GAP_X = NODE.gap.horizontal
const GAP_Y = NODE.gap.vertical

/**
 * Simple top-to-bottom DAG layout for WAC workflow graphs.
 * Uses the same NODE dimensions as the flow editor for visual consistency.
 */
export function dagToXyflow(dag: WacWorkflowDag): { nodes: Node[]; edges: Edge[] } {
	if (dag.nodes.length === 0) {
		return { nodes: [], edges: [] }
	}

	// Build adjacency maps
	const childrenMap = new Map<string, { id: string; label?: string }[]>()
	const parentMap = new Map<string, string[]>()

	for (const edge of dag.edges) {
		if (!childrenMap.has(edge.from)) childrenMap.set(edge.from, [])
		childrenMap.get(edge.from)!.push({ id: edge.to, label: edge.label })
		if (!parentMap.has(edge.to)) parentMap.set(edge.to, [])
		parentMap.get(edge.to)!.push(edge.from)
	}

	// Find root nodes (no parents, excluding back-edges to loop starts)
	const roots = dag.nodes.filter((n) => {
		const parents = parentMap.get(n.id) ?? []
		return (
			parents.length === 0 ||
			parents.every((p) => {
				const edge = dag.edges.find((e) => e.from === p && e.to === n.id)
				return edge?.label === 'next'
			})
		)
	})

	// Assign layers using BFS (ignoring back-edges)
	const layers = new Map<string, number>()
	const queue: string[] = []

	for (const root of roots) {
		layers.set(root.id, 0)
		queue.push(root.id)
	}

	while (queue.length > 0) {
		const nodeId = queue.shift()!
		const layer = layers.get(nodeId)!
		const children = childrenMap.get(nodeId) ?? []

		for (const child of children) {
			if (child.label === 'next') continue // skip back-edges
			const existing = layers.get(child.id)
			if (existing === undefined || existing < layer + 1) {
				layers.set(child.id, layer + 1)
				queue.push(child.id)
			}
		}
	}

	// Group nodes by layer
	const layerGroups = new Map<number, string[]>()
	for (const [nodeId, layer] of layers) {
		if (!layerGroups.has(layer)) layerGroups.set(layer, [])
		layerGroups.get(layer)!.push(nodeId)
	}

	const maxLayer = Math.max(...layers.values(), 0)

	// Position nodes — centered, using flow editor dimensions
	const positions = new Map<string, { x: number; y: number }>()

	for (let layer = 0; layer <= maxLayer; layer++) {
		const group = layerGroups.get(layer) ?? []
		const totalWidth = group.length * NODE.width + (group.length - 1) * GAP_X
		const startX = -totalWidth / 2

		for (let i = 0; i < group.length; i++) {
			positions.set(group[i], {
				x: startX + i * (NODE.width + GAP_X),
				y: layer * (NODE.height + GAP_Y)
			})
		}
	}

	// Convert to xyflow nodes
	const nodeMap = new Map(dag.nodes.map((n) => [n.id, n]))
	const xyNodes: Node[] = []

	for (const [id, pos] of positions) {
		const dagNode = nodeMap.get(id)
		if (!dagNode) continue

		xyNodes.push({
			id,
			type: getXyflowNodeType(dagNode),
			position: { x: pos.x, y: pos.y },
			data: { dagNode },
			width: NODE.width,
			height: NODE.height
		})
	}

	// Convert to xyflow edges
	const xyEdges: Edge[] = dag.edges.map((e, i) => ({
		id: `e-${i}`,
		source: e.from,
		target: e.to,
		type: 'wacEdge',
		label: e.label === 'next' ? '' : (e.label ?? ''),
		animated: e.label === 'next',
		style: e.label === 'next' ? 'stroke-dasharray: 5 5;' : undefined
	}))

	return { nodes: xyNodes, edges: xyEdges }
}

function getXyflowNodeType(node: WacDagNode): string {
	switch (node.node_type.type) {
		case 'Step':
		case 'InlineStep':
			return 'wacStep'
		case 'Sleep':
		case 'WaitForApproval':
		case 'Branch':
		case 'ParallelStart':
		case 'ParallelEnd':
		case 'LoopStart':
		case 'LoopEnd':
		case 'Merge':
		case 'Return':
			return 'wacControl'
		default:
			return 'wacStep'
	}
}
