import type { RichConfiguration } from '$lib/components/apps/types'
import { getNextId } from '$lib/components/flows/idUtils'
import type { DecisionTreeNode } from '../../component'

function createBooleanRC(): RichConfiguration {
	return {
		type: 'evalv2',
		expr: 'true',
		fieldType: 'boolean',
		connections: []
	}
}

export function addNode(nodes: DecisionTreeNode[], sourceNode: DecisionTreeNode | undefined) {
	const nextId = getNextId(nodes.map((node) => node.id))

	if (sourceNode) {
		const newNode: DecisionTreeNode = {
			id: nextId,
			label: nextId,
			next: sourceNode.next,
			allowed: createBooleanRC()
		}

		nodes.push(newNode)

		nodes = nodes.map((node) => {
			if (node.id === sourceNode?.id) {
				node.next = [
					{
						id: newNode.id,
						condition: createBooleanRC()
					}
				]
			}
			return node
		})

		return nodes
	} else {
		const firstNode = getFirstNode(nodes)

		if (firstNode) {
			const newNode: DecisionTreeNode = {
				id: nextId,
				label: nextId,
				next: [{ id: firstNode.id, condition: createBooleanRC() }],
				allowed: createBooleanRC()
			}

			nodes.push(newNode)
		}
		return nodes
	}
}

export function insertNode(
	nodes: DecisionTreeNode[],
	parentId: string,
	sourceNode: DecisionTreeNode
) {
	const nextId = getNextId(nodes.map((node) => node.id))

	const newNode: DecisionTreeNode = {
		id: nextId,
		label: nextId,
		next: [
			{
				id: sourceNode.id,
				condition: createBooleanRC()
			}
		],
		allowed: createBooleanRC()
	}

	nodes = [...nodes, newNode]

	nodes = nodes.map((node) => {
		if (node.id === parentId) {
			node.next = node.next.map((next) => {
				if (next.id === sourceNode.id) {
					return {
						id: newNode.id,
						condition: createBooleanRC()
					}
				}
				return next
			})
		}
		return node
	})

	return nodes
}

export function removeNode(
	nodes: DecisionTreeNode[],
	nodeToRemove: DecisionTreeNode | undefined
): DecisionTreeNode[] {
	if (!nodeToRemove) {
		return nodes
	}

	const parentNodes = nodes.filter((n) => n.next.some((next) => next.id === nodeToRemove.id))

	if (parentNodes.length == 0) {
		// Case when there is no parent node
		if (nodeToRemove.next.length === 1) {
			return nodes.filter((n) => n.id !== nodeToRemove.id)
		}
	} else {
		parentNodes.forEach((parentNode, index) => {
			if (parentNode.next.length === 1) {
				// Parent has only one next node
				parentNode.next = nodeToRemove.next
			} else if (parentNode.next.length > 1) {
				// Parent has multiple next nodes
				parentNode.next = parentNode.next
					.filter((next) => next.id !== nodeToRemove.id)
					.concat(nodeToRemove.next)

				// Remove duplicates
				parentNode.next = parentNode.next.filter(
					(next, index, self) => self.findIndex((t) => t.id === next.id) === index
				)
			}
		})

		nodes = nodes.filter((n) => n.id !== nodeToRemove.id)
	}

	return nodes
}

export function removeBranch(
	nodes: DecisionTreeNode[],
	firstNodeInBranch: DecisionTreeNode | undefined,
	parentNodeId: string,
	onRemove: (id: string) => void
) {
	const parentNode = nodes.find((n) => n.id === parentNodeId)
	const collapseNodeId = findCollapseNode(nodes, parentNodeId)

	if (!parentNode || !collapseNodeId || !firstNodeInBranch) {
		return nodes
	}

	const firstNodeInBranchParent = getParents(nodes, firstNodeInBranch.id)
	parentNode.next = parentNode.next.filter((next) => next.id !== firstNodeInBranch?.id)

	if (firstNodeInBranchParent.length > 1) {
		return nodes
	}

	if (firstNodeInBranch && firstNodeInBranch.id !== collapseNodeId) {
		const visited = new Set<string>()
		const dfs = (nodeId: string) => {
			if (visited.has(nodeId)) return
			visited.add(nodeId)

			const node = nodes.find((n) => n.id === nodeId)
			if (node) {
				node.next.forEach((next) => {
					if (next.id !== collapseNodeId) {
						dfs(next.id)
					}
				})

				onRemove(node.id)
				nodes = nodes.filter((n) => n.id !== nodeId)
			}
		}

		dfs(firstNodeInBranch.id)
	}

	if (firstNodeInBranch && !firstNodeInBranch.next.length) {
		onRemove(firstNodeInBranch.id)
		nodes = nodes.filter((n) => n.id !== firstNodeInBranch.id)
	}

	return nodes
}

export function findCollapseNode(tree: DecisionTreeNode[], startId: string): string | null {
	const nodeMap = new Map<string, DecisionTreeNode>()
	tree.forEach((node) => nodeMap.set(node.id, node))

	let paths: string[][] = []

	const dfs = (nodeId: string, path: string[]) => {
		if (nodeMap.has(nodeId)) {
			path.push(nodeId)
			const node = nodeMap.get(nodeId)!
			if (node.next && node.next.length > 0) {
				node.next.forEach((nextNode) => dfs(nextNode.id, [...path]))
			} else {
				paths.push(path)
			}
		}
	}

	dfs(startId, [])

	paths = paths.map((path) => path.slice(1))

	return findFirstCommonLetter(paths)
}

function findFirstCommonLetter(arrays: string[][]): string | null {
	const first = arrays[0]
	const rest = arrays.slice(1)

	for (let i = 0; i < first?.length; i++) {
		const letter = first[i]

		if (rest.every((array) => array.includes(letter))) {
			return letter
		}
	}

	return null
}

function createNewNode(nodes: DecisionTreeNode[], id: string) {
	const nextId = getNextId(nodes.map((node) => node.id))

	const newNode: DecisionTreeNode = {
		id: nextId,
		label: nextId,
		next: [
			{
				id: id ?? '',
				condition: createBooleanRC()
			}
		],
		allowed: createBooleanRC()
	}

	return newNode
}

export function addNewBranch(nodes: DecisionTreeNode[], startNode: DecisionTreeNode) {
	const collapseNode = findCollapseNode(nodes, startNode.id)

	if (!collapseNode) {
		return nodes
	}

	const newNode = createNewNode(nodes, collapseNode)
	nodes.push(newNode)

	nodes = nodes.map((node) => {
		if (node.id === startNode?.id) {
			node.next.push({
				id: newNode.id,
				condition: createBooleanRC()
			})
		}
		return node
	})

	return nodes
}

export function getParents(nodes: DecisionTreeNode[], nodeId: string): string[] {
	let parentIds: string[] = []

	nodes.forEach((n) => {
		n.next.forEach((nextNode) => {
			if (nextNode.id == nodeId) {
				parentIds.push(n.id)
			}
		})
	})

	const firstNode = getFirstNode(nodes)
	// if first node, add start node as parent
	if (nodeId == firstNode?.id) {
		parentIds.push('start')
	}

	return parentIds
}

export function getFirstNode(nodes: DecisionTreeNode[]): DecisionTreeNode | undefined {
	// No other nodes has this node as next
	return nodes.find((node) => !nodes.some((n) => n.next.some((next) => next.id === node.id)))
}

export function isDebugging(debuggingComponents: Record<string, number>, id: string): boolean {
	return Object.keys(debuggingComponents).includes(id)
}
