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

export function addNode(nodes: DecisionTreeNode[], sourceNode: DecisionTreeNode) {
	const nextId = getNextId(nodes.map((node) => node.id))

	const newNode: DecisionTreeNode = {
		id: nextId,
		label: nextId,
		next: sourceNode.next,
		allowed: createBooleanRC()
	}

	nodes.push(newNode)

	nodes = nodes.map((node) => {
		if (node.id === sourceNode.id) {
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
}

export function addBranch(nodes: DecisionTreeNode[], sourceNode: DecisionTreeNode) {
	const nextId = getNextId(nodes.map((node) => node.id))

	const left: DecisionTreeNode = {
		id: nextId,
		label: nextId,
		next: sourceNode.next,
		allowed: createBooleanRC()
	}

	const rightNextId = getNextId([nextId, ...nodes.map((node) => node.id)])

	const right: DecisionTreeNode = {
		id: rightNextId,
		label: rightNextId,
		next: sourceNode.next,
		allowed: createBooleanRC()
	}

	nodes.push(left)
	nodes.push(right)

	nodes = nodes.map((node) => {
		if (node.id === sourceNode.id) {
			node.next = [
				{
					id: left.id,
					condition: createBooleanRC()
				},
				{
					id: right.id,
					condition: createBooleanRC()
				}
			]
		}
		return node
	})

	return nodes
}

export function removeNode(nodes: DecisionTreeNode[], nodeToRemove: DecisionTreeNode | undefined) {
	if (!nodeToRemove) {
		return nodes
	}

	const parentNode = nodes.find((n) => n.next.find((next) => next.id == nodeToRemove.id))

	if (!parentNode && nodeToRemove.next.length == 1) {
		nodes = nodes.filter((n) => n.id != nodeToRemove.id)
	} else if (parentNode && parentNode?.next.length === 1) {
		parentNode.next = nodeToRemove.next

		nodes = nodes.filter((n) => n.id != nodeToRemove.id)
	} else if (parentNode && parentNode?.next.length > 1) {
		parentNode.next = parentNode.next.filter((next) => next.id != nodeToRemove.id)
		parentNode.next = [...parentNode.next, ...nodeToRemove.next]

		// Filter out duplicates
		parentNode.next = parentNode.next.filter(
			(next, index, self) => index === self.findIndex((t) => t.id === next.id)
		)

		nodes = nodes.filter((n) => n.id != nodeToRemove.id)
	}

	return nodes
}

export function removeBranch(
	nodes: DecisionTreeNode[],
	firsNodeInBranch: DecisionTreeNode | undefined,
	parentNodeId: string
) {
	const parentNode = nodes.find((n) => n.id == parentNodeId)
	const collapseNode = findCollapseNode(nodes, parentNodeId)

	if (!parentNode || !collapseNode) {
		return nodes
	}

	parentNode.next = parentNode.next.filter((next) => next.id != firsNodeInBranch?.id)

	if (firsNodeInBranch && firsNodeInBranch.id !== collapseNode) {
		// Iterate over all subnodes and remove them until we reach the collapse node
		const dfs = (nodeId: string) => {
			const node = nodes.find((n) => n.id == nodeId)
			if (node) {
				node.next.forEach((next) => {
					if (next.id !== collapseNode) {
						dfs(next.id)
					}
				})
				nodes = nodes.filter((n) => n.id != nodeId)
			}
		}
		dfs(firsNodeInBranch?.id)
	}

	return nodes
}

function findCollapseNode(tree: DecisionTreeNode[], startId: string): string | null {
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

function findFirstCommonLetter(arrays) {
	const first = arrays[0]
	const rest = arrays.slice(1)

	for (let i = 0; i < first.length; i++) {
		const letter = first[i]

		if (rest.every((array) => array.includes(letter))) {
			return letter
		}
	}

	return null
}

export function insertFirstNode(nodes: DecisionTreeNode[]) {
	const firstNode = nodes[0]
	nodes.unshift(createNewNode(nodes, firstNode?.id))
	return nodes
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
