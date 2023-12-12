import type { RichConfiguration } from '$lib/components/apps/types'
import { getNextId } from '$lib/components/flows/idUtils'
import type { DecisionTreeNode } from '../../component'

export function addNode(nodes: DecisionTreeNode[], sourceNode: DecisionTreeNode) {
	const nextId = getNextId(nodes.map((node) => node.id))

	const newNode: DecisionTreeNode = {
		id: nextId,
		label: nextId,
		next: sourceNode.next,
		required: {
			type: 'evalv2',
			expr: 'true',
			fieldType: 'boolean'
		} as RichConfiguration
	}

	nodes.push(newNode)

	nodes = nodes.map((node) => {
		if (node.id === sourceNode.id) {
			node.next = [
				{
					id: newNode.id,
					condition: {
						type: 'evalv2',
						expr: 'true',
						fieldType: 'boolean'
					} as RichConfiguration
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
		required: {
			type: 'evalv2',
			expr: 'true',
			fieldType: 'boolean'
		} as RichConfiguration
	}

	const rightNextId = getNextId([nextId, ...nodes.map((node) => node.id)])

	const right: DecisionTreeNode = {
		id: rightNextId,
		label: rightNextId,
		next: sourceNode.next,
		required: {
			type: 'evalv2',
			expr: 'true',
			fieldType: 'boolean'
		} as RichConfiguration
	}

	nodes.push(left)
	nodes.push(right)

	nodes = nodes.map((node) => {
		if (node.id === sourceNode.id) {
			node.next = [
				{
					id: left.id,
					condition: {
						type: 'evalv2',
						expr: 'true',
						fieldType: 'boolean'
					} as RichConfiguration
				},
				{
					id: right.id,
					condition: {
						type: 'evalv2',
						expr: 'true',
						fieldType: 'boolean'
					} as RichConfiguration
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

		return nodes
	}

	if (parentNode?.next.length == 1) {
		parentNode.next = nodeToRemove.next

		nodes = nodes.filter((n) => n.id != nodeToRemove.id)
	}

	if (parentNode && parentNode?.next.length > 1) {
		const collapseNode = findCollapseNode(nodes, parentNode.id)

		if (nodeToRemove.next.find((next) => next.id == collapseNode)) {
			parentNode.next = parentNode.next.filter((next) => next.id != nodeToRemove.id)
			nodes = nodes.filter((n) => n.id != nodeToRemove.id)
		} else {
			parentNode.next = parentNode.next.filter((next) => next.id != nodeToRemove.id)
			nodeToRemove.next.forEach((next) => {
				parentNode.next.push(next)
			})
			nodes = nodes.filter((n) => n.id != nodeToRemove.id)
		}
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
	const nextId = getNextId(nodes.map((node) => node.id))

	const newNode: DecisionTreeNode = {
		id: nextId,
		label: nextId,
		next: [
			{
				id: firstNode?.id ?? '',
				condition: {
					type: 'evalv2',
					expr: 'true',
					fieldType: 'boolean'
				} as RichConfiguration
			}
		],
		required: {
			type: 'evalv2',
			expr: 'true',
			fieldType: 'boolean'
		} as RichConfiguration
	}

	nodes.unshift(newNode)

	return nodes
}

export function addNewBranch(nodes: DecisionTreeNode[], startNode: DecisionTreeNode) {
	const nextId = getNextId(nodes.map((node) => node.id))

	const collapseNode = findCollapseNode(nodes, startNode.id)

	const newNode: DecisionTreeNode = {
		id: nextId,
		label: nextId,
		next: [
			{
				id: collapseNode ?? '',
				condition: {
					type: 'evalv2',
					expr: 'true',
					fieldType: 'boolean'
				} as RichConfiguration
			}
		],
		required: {
			type: 'evalv2',
			expr: 'true',
			fieldType: 'boolean'
		} as RichConfiguration
	}

	nodes.push(newNode)

	nodes = nodes.map((node) => {
		if (node.id === startNode?.id) {
			node.next.push({
				id: newNode.id,
				condition: {
					type: 'evalv2',
					expr: 'true',
					fieldType: 'boolean'
				} as RichConfiguration
			})
		}
		return node
	})

	return nodes
}
