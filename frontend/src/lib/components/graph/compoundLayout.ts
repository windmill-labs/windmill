import { sugiyama, dagStratify, coordCenter, decrossTwoLayer, decrossOpt } from 'd3-dag'
import { NODE } from './util'

type LayoutNode = {
	id: string
	parentIds?: string[]
}

type LayoutConstants = {
	nodeWidth: number
	nodeHeight: number
	gapH: number
	gapV: number
}

type CompoundGroup = {
	type: 'branch' | 'loop'
	headId: string
	endId: string
	branches: {
		labelId: string
		innerIds: string[]
	}[]
}

type LayoutResult = {
	positions: Map<string, { x: number; y: number }>
	bbox: { width: number; height: number }
	contentMinX: number
}

const LOOP_INDENT = 25

/**
 * Detect compound groups from a flat list of node IDs.
 * Uses ID naming conventions from graphBuilder:
 * - BranchAll/BranchOne: node X has children X-branch-N and X-end
 * - ForLoop/WhileLoop: node X has child X-start and X-end
 */
function detectGroups(
	nodeIds: Set<string>,
	allNodes: Map<string, LayoutNode>,
	childrenMap: Map<string, string[]>
): CompoundGroup[] {
	const groups: CompoundGroup[] = []

	for (const id of nodeIds) {
		if (!id.endsWith('-end')) continue

		// Extract base ID (everything before -end)
		const baseId = id.slice(0, -4)
		if (!nodeIds.has(baseId)) continue

		const baseNode = allNodes.get(baseId)
		if (!baseNode) continue

		// Check for branch pattern: probe for baseId-branch-N nodes directly
		const branchLabelIds: string[] = []
		if (nodeIds.has(`${baseId}-branch-default`)) {
			branchLabelIds.push(`${baseId}-branch-default`)
		}
		for (let i = 0; nodeIds.has(`${baseId}-branch-${i}`); i++) {
			branchLabelIds.push(`${baseId}-branch-${i}`)
		}

		// Check for loop pattern: baseId-start node
		const hasStart = nodeIds.has(`${baseId}-start`)

		if (branchLabelIds.length > 0) {
			// Branches are already in correct order: default first, then 0, 1, 2...
			const branches = branchLabelIds.map((labelId) => ({
				labelId,
				innerIds: findInnerIds(labelId, id, nodeIds, childrenMap)
			}))

			groups.push({ type: 'branch', headId: baseId, endId: id, branches })
		} else if (hasStart) {
			const innerIds = findInnerIds(`${baseId}-start`, id, nodeIds, childrenMap)

			groups.push({
				type: 'loop',
				headId: baseId,
				endId: id,
				branches: [{ labelId: `${baseId}-start`, innerIds }]
			})
		}
	}

	return groups
}

/**
 * Find inner node IDs between a label/start node and an end node.
 * These are nodes that are reachable from the label node but not including
 * the label or end node themselves.
 */
function findInnerIds(
	labelId: string,
	endId: string,
	nodeIds: Set<string>,
	childrenMap: Map<string, string[]>
): string[] {
	const inner: string[] = []
	const visited = new Set<string>()

	// BFS from label to find all reachable nodes before end
	const queue = [labelId]
	visited.add(labelId)
	visited.add(endId) // Don't traverse past end

	let qi = 0
	while (qi < queue.length) {
		const current = queue[qi++]
		const kids = childrenMap.get(current) ?? []
		for (const kid of kids) {
			if (visited.has(kid)) continue
			if (!nodeIds.has(kid)) continue
			visited.add(kid)
			inner.push(kid)
			queue.push(kid)
		}
	}

	return inner
}

/**
 * Run sugiyama layout on a set of nodes with parent relationships.
 * Returns x,y positions for each node, centered at x=0.
 */
function runSugiyama(
	nodes: { id: string; parentIds?: string[] }[],
	constants: LayoutConstants,
	nodeSizes?: Map<string, { width: number; height: number }>
): { positions: Map<string, { x: number; y: number }>; width: number; height: number } {
	if (nodes.length === 0) {
		return { positions: new Map(), width: 0, height: 0 }
	}

	if (nodes.length === 1) {
		const pos = new Map<string, { x: number; y: number }>()
		pos.set(nodes[0].id, { x: 0, y: 0 })
		const w = nodeSizes?.get(nodes[0].id)?.width ?? constants.nodeWidth
		const h = nodeSizes?.get(nodes[0].id)?.height ?? constants.nodeHeight
		return { positions: pos, width: w, height: h }
	}

	const nodeIdSet = new Set(nodes.map((n) => n.id))
	const dagNodes = nodes.map((n) => ({
		id: n.id,
		parentIds: (n.parentIds ?? []).filter((pid) => nodeIdSet.has(pid))
	}))

	const dag = dagStratify().id(({ id }: { id: string }) => id)(dagNodes)

	let boxSize: { width: number; height: number }
	try {
		const layout = sugiyama()
			.decross(nodes.length > 20 ? decrossTwoLayer() : decrossOpt())
			.coord(coordCenter())
			.nodeSize((d: any) => {
				const nodeId = d?.data?.id ?? ''
				const size = nodeSizes?.get(nodeId)
				const w = size?.width ?? constants.nodeWidth
				const h = size?.height ?? constants.nodeHeight
				return [w + constants.gapH, h + constants.gapV] as readonly [number, number]
			})
		boxSize = layout(dag as any) as any
	} catch {
		const layout = sugiyama()
			.decross(decrossTwoLayer())
			.coord(coordCenter())
			.nodeSize((d: any) => {
				const nodeId = d?.data?.id ?? ''
				const size = nodeSizes?.get(nodeId)
				const h = size?.height ?? constants.nodeHeight
				return [constants.nodeWidth + constants.gapH, h + constants.gapV]
			})
		boxSize = layout(dag as any) as any
	}

	const positions = new Map<string, { x: number; y: number }>()
	for (const desc of dag.descendants()) {
		const nodeId = desc.data.id
		// sugiyama returns CENTER positions; convert to TOP by subtracting half the node's allocated height
		const h = nodeSizes?.get(nodeId)?.height ?? constants.nodeHeight
		const rawY = (desc as any).y ?? 0
		positions.set(nodeId, {
			x: (desc as any).x ?? 0,
			y: rawY - (h + constants.gapV) / 2
		})
	}

	// Normalize y so minimum = 0
	let minY = Infinity
	for (const pos of positions.values()) {
		minY = Math.min(minY, pos.y)
	}
	if (minY !== Infinity && minY !== 0) {
		for (const pos of positions.values()) {
			pos.y -= minY
		}
	}

	// Normalize x so center of bbox = 0 (important for nested branch placement)
	let minX = Infinity
	let maxX = -Infinity
	for (const pos of positions.values()) {
		minX = Math.min(minX, pos.x)
		maxX = Math.max(maxX, pos.x)
	}
	if (minX !== Infinity) {
		const centerX = (minX + maxX) / 2
		for (const pos of positions.values()) {
			pos.x -= centerX
		}
	}

	return { positions, width: boxSize.width, height: boxSize.height }
}

/**
 * Recursive compound layout.
 *
 * 1. Detect compound groups at this level
 * 2. For each group, recursively lay out each branch
 * 3. Compute wrapper bbox for each group
 * 4. Replace group nodes with a single wrapper pseudo-node
 * 5. Run sugiyama on the simplified graph
 * 6. Expand wrapper positions back to absolute positions
 */
function layoutLevel(
	nodeIds: string[],
	allNodes: Map<string, LayoutNode>,
	constants: LayoutConstants,
	childrenMap: Map<string, string[]>
): LayoutResult {
	const positions = new Map<string, { x: number; y: number }>()
	const nodeIdSet = new Set(nodeIds)

	if (nodeIds.length === 0) {
		return {
			positions,
			bbox: { width: constants.nodeWidth, height: 0 },
			contentMinX: 0
		}
	}

	// Step 1: detect compound groups at this level
	const groups = detectGroups(nodeIdSet, allNodes, childrenMap)

	// First pass: quick set of ALL group-owned inner IDs (just for filtering nested heads)
	const allGroupOwnedIds = new Set<string>()
	for (const group of groups) {
		if (!nodeIdSet.has(group.headId)) continue
		for (const branch of group.branches) {
			for (const innerId of branch.innerIds) {
				allGroupOwnedIds.add(innerId)
			}
		}
	}

	// Filter to top-level groups (head not owned by another group)
	const topLevelGroups = groups.filter(
		(g) => nodeIdSet.has(g.headId) && !allGroupOwnedIds.has(g.headId)
	)

	// Build final groupOwnedIds and groupByHeadId from top-level only
	const groupOwnedIds = new Set<string>()
	const groupByHeadId = new Map<string, CompoundGroup>()
	for (const group of topLevelGroups) {
		groupByHeadId.set(group.headId, group)
		groupOwnedIds.add(group.endId)
		for (const branch of group.branches) {
			groupOwnedIds.add(branch.labelId)
			for (const innerId of branch.innerIds) {
				groupOwnedIds.add(innerId)
			}
		}
	}

	// Step 2-3: Recursively lay out each group and compute wrapper sizes
	type GroupLayout = {
		group: CompoundGroup
		branchLayouts: {
			labelId: string
			result: LayoutResult
			bbox: { width: number; height: number }
		}[]
		branchWidths: number[]
		totalWidth: number
		wrapperWidth: number
		wrapperHeight: number
		maxBranchHeight: number
	}

	const groupLayouts = new Map<string, GroupLayout>()
	const wrapperSizes = new Map<string, { width: number; height: number }>()

	for (const group of topLevelGroups) {
		const branchLayouts: GroupLayout['branchLayouts'] = []
		const isBranch = group.type === 'branch'

		for (const branch of group.branches) {
			const branchNodeIds = [branch.labelId, ...branch.innerIds]

			// Find sub-groups within this branch
			const result = layoutLevel(branchNodeIds, allNodes, constants, childrenMap)

			branchLayouts.push({
				labelId: branch.labelId,
				result,
				bbox: result.bbox
			})
		}

		// Compute wrapper dimensions
		let wrapperWidth: number
		let wrapperHeight: number
		let branchWidths: number[] = []
		let totalWidth = 0
		let maxBranchHeight = 0
		const rowHeight = constants.nodeHeight + constants.gapV

		if (isBranch) {
			// Place branches side by side horizontally
			branchWidths = branchLayouts.map((bl) =>
				Math.max(bl.bbox.width, constants.nodeWidth)
			)
			const gaps = Math.max(0, branchWidths.length - 1) * constants.gapH
			totalWidth =
				branchWidths.reduce((s, w) => s + w, 0) + gaps
			wrapperWidth = Math.max(totalWidth, constants.nodeWidth)

			maxBranchHeight = Math.max(0, ...branchLayouts.map((bl) => bl.bbox.height))
			// head row + branch content + end row
			wrapperHeight = rowHeight + maxBranchHeight + rowHeight
		} else {
			// Loop: body is indented
			const bodyWidth = branchLayouts[0]?.bbox.width ?? constants.nodeWidth
			const bodyHeight = branchLayouts[0]?.bbox.height ?? 0
			wrapperWidth = Math.max(bodyWidth + LOOP_INDENT * 2, constants.nodeWidth)
			// head row + start row + body + end row
			wrapperHeight = rowHeight + bodyHeight + rowHeight
		}
		groupLayouts.set(group.headId, {
			group,
			branchLayouts,
			branchWidths,
			totalWidth,
			wrapperWidth,
			wrapperHeight,
			maxBranchHeight
		})
		wrapperSizes.set(group.headId, { width: wrapperWidth, height: wrapperHeight })
	}

	// Step 4: Build flattened node list for sugiyama
	// Merge into a single pass: create flatNode and compute final parentIds with end→head redirection
	const endToHead = new Map<string, string>()
	for (const group of topLevelGroups) {
		endToHead.set(group.endId, group.headId)
	}

	const flatNodes: { id: string; parentIds?: string[] }[] = []
	for (const nid of nodeIds) {
		if (groupOwnedIds.has(nid)) continue

		const originalNode = allNodes.get(nid)!
		const seen = new Set<string>()
		const newParents: string[] = []
		for (const pid of originalNode.parentIds ?? []) {
			if (!nodeIdSet.has(pid)) continue
			const resolved = endToHead.get(pid) ?? (groupOwnedIds.has(pid) ? undefined : pid)
			if (resolved && !seen.has(resolved)) {
				seen.add(resolved)
				newParents.push(resolved)
			}
		}
		flatNodes.push({ id: nid, parentIds: newParents })
	}

	// Step 5: Run sugiyama on flattened nodes
	const sugResult = runSugiyama(flatNodes, constants, wrapperSizes)

	// Step 6: Resolve absolute positions
	// First, set positions for regular (non-group) nodes
	for (const [nid, pos] of sugResult.positions) {
		if (groupByHeadId.has(nid)) continue // Handle groups separately
		positions.set(nid, { x: pos.x, y: pos.y })
	}

	// Now expand group wrappers into absolute positions
	for (const [headId, gl] of groupLayouts) {
		const wrapperPos = sugResult.positions.get(headId)
		if (!wrapperPos) continue

		const rowHeight = constants.nodeHeight + constants.gapV
		const isBranch = gl.group.type === 'branch'

		// Position the head node at the top-center of the wrapper
		positions.set(headId, { x: wrapperPos.x, y: wrapperPos.y })

		if (isBranch) {
			// Reuse cached branchWidths and totalWidth
			let currentX = wrapperPos.x - gl.totalWidth / 2

			for (let bi = 0; bi < gl.branchLayouts.length; bi++) {
				const bl = gl.branchLayouts[bi]
				const bw = gl.branchWidths[bi]
				const branchCenterX = currentX + bw / 2

				// Offset all branch positions relative to the branch center
				for (const [innerNodeId, innerPos] of bl.result.positions) {
					positions.set(innerNodeId, {
						x: branchCenterX + innerPos.x,
						y: wrapperPos.y + rowHeight + innerPos.y
					})
				}

				currentX += bw + constants.gapH
			}

			// Position end node below all branches
			const maxBranchHeight = gl.maxBranchHeight
			positions.set(gl.group.endId, {
				x: wrapperPos.x,
				y: wrapperPos.y + rowHeight + maxBranchHeight + constants.gapV
			})
		} else {
			// Loop: position start, body, and end
			const bl = gl.branchLayouts[0]
			if (bl) {
				// Position body nodes with indent
				for (const [innerNodeId, innerPos] of bl.result.positions) {
					positions.set(innerNodeId, {
						x: wrapperPos.x + LOOP_INDENT + innerPos.x,
						y: wrapperPos.y + rowHeight + innerPos.y
					})
				}
			}

			// Position end node below body
			const bodyHeight = bl?.bbox.height ?? 0
			positions.set(gl.group.endId, {
				x: wrapperPos.x,
				y: wrapperPos.y + rowHeight + bodyHeight + constants.gapV
			})
		}
	}

	// Compute overall bbox (nodes + group wrapper extents)
	let minX = Infinity
	let maxX = -Infinity
	let minY = Infinity
	let maxY = 0
	for (const pos of positions.values()) {
		minX = Math.min(minX, pos.x - constants.nodeWidth / 2)
		maxX = Math.max(maxX, pos.x + constants.nodeWidth / 2)
		minY = Math.min(minY, pos.y)
		maxY = Math.max(maxY, pos.y + constants.nodeHeight)
	}
	// Account for group wrapper extents in bbox (e.g. LOOP_INDENT makes wrappers wider than nodes)
	for (const [headId, gl] of groupLayouts) {
		const pos = sugResult.positions.get(headId)
		if (!pos) continue
		minX = Math.min(minX, pos.x - gl.wrapperWidth / 2)
		maxX = Math.max(maxX, pos.x + gl.wrapperWidth / 2)
		maxY = Math.max(maxY, pos.y + gl.wrapperHeight)
	}

	const contentMinX = minX === Infinity ? 0 : minX

	const bboxWidth = maxX - minX
	const bboxHeight = maxY - (positions.size > 0 ? minY : 0)

	const finalBbox = {
		width: Math.max(bboxWidth, constants.nodeWidth),
		height: Math.max(bboxHeight, 0)
	}
	return { positions, bbox: finalBbox, contentMinX }
}

/**
 * Main entry point for compound layout.
 *
 * Takes the flat list of nodes and edges from graphBuilder and produces
 * absolute positions that account for compound structure (branches, loops).
 */
export function compoundLayout(
	nodes: { id: string; parentIds?: string[] }[],
	constants?: Partial<LayoutConstants>
): LayoutResult {
	const c: LayoutConstants = {
		nodeWidth: constants?.nodeWidth ?? NODE.width,
		nodeHeight: constants?.nodeHeight ?? NODE.height,
		gapH: constants?.gapH ?? NODE.gap.horizontal,
		gapV: constants?.gapV ?? NODE.gap.vertical
	}

	// Build node map
	const allNodes = new Map<string, LayoutNode>()
	for (const n of nodes) {
		allNodes.set(n.id, n)
	}

	// Build children map once (reverse of parentIds), shared across all recursion levels
	const childrenMap = new Map<string, string[]>()
	for (const [nid, node] of allNodes) {
		for (const pid of node.parentIds ?? []) {
			if (!childrenMap.has(pid)) childrenMap.set(pid, [])
			childrenMap.get(pid)!.push(nid)
		}
	}

	const nodeIds = nodes.map((n) => n.id)
	const result = layoutLevel(nodeIds, allNodes, c, childrenMap)

	// Shift positions so minX=0 (left-aligned).
	// FlowGraphV2 centers with: xCenter = viewport/2 - bbox.width/2
	// which assumes positions start at x=0.
	if (result.positions.size > 0) {
		const minX = result.contentMinX
		if (minX !== 0 && minX !== Infinity) {
			for (const pos of result.positions.values()) {
				pos.x -= minX
			}
		}
	}

	// Check for missing nodes
	const missing = nodes.filter((n) => !result.positions.has(n.id))
	if (missing.length > 0) {
		console.warn(
			'[compoundLayout] MISSING positions for:',
			missing.map((n) => n.id)
		)
	}

	return result
}
