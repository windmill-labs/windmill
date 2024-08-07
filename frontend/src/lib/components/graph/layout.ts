import {
	type Node
	// @ts-ignore
} from '@xyflow/svelte'
import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
import { NODE } from './util'

export function layoutNodes(
	nodes: Node[],
	fullSize: number,
	fullWidth: number,
	width: number
): { nodes: Node[] } {
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
		nodes: newNodes
	}
}
