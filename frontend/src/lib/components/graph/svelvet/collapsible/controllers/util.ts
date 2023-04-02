import type { EdgeType, NodeType, ResizeNodeType, StoreType } from '../../store/types/types'
import type { UserEdgeType, UserNodeType } from '../../types/types'
import { Collapsible } from '../models/Collapsible'
import type { CollapsibleType } from '../types/types'
import { get } from 'svelte/store'
import type { AnchorType } from '../../edges/types/types'
import { getAnchorById } from '../../edges/controllers/util'
import { getAnchors } from '../../edges/controllers/util'

const pkStringGenerator = () => (Math.random() + 1).toString(36).substring(7)

/*
Initializes store with array of Collapsible objects. You shoould only use this if you want the collapsible feature enabled.
*/
export function populateCollapsibleStore(
	store: StoreType,
	userNodes: UserNodeType[],
	userEdges: UserEdgeType[],
	canvasId: string
) {
	const newCollapsibleStore: CollapsibleType[] = []
	for (let userNode of userNodes) {
		const collapsible = new Collapsible(pkStringGenerator(), userNode.id, 0, 'expanded')
		newCollapsibleStore.push(collapsible)
	}
	store.collapsibleStore.set(newCollapsibleStore)
}

// Given a nodeId, find ids of all connecting target nodes
function findTargets(store: StoreType, nodeId: string): string[] {
	// get source anchors on the node
	const anchors = getAnchors(store, {
		nodeId: nodeId,
		sourceOrTarget: 'source'
	})

	// get target anchors on other node, and record the node id
	const targetNodeIds: string[] = []
	for (const anchor of anchors) {
		const targetAnchorId = anchor.getOtherAnchorId()
		const targetAnchor = getAnchorById(store, targetAnchorId)
		targetNodeIds.push(targetAnchor.nodeId)
	}
	return targetNodeIds
}

// traverses tree and increments hideCount
function traverseAndIncrement(
	store: StoreType,
	nodeId: string,
	operation: 'increment' | 'decrement'
) {
	const collapsibles = get(store.collapsibleStore)
	recursiveTraverse(nodeId)
	store.collapsibleStore.set(collapsibles)

	function recursiveTraverse(nId: string) {
		for (const collapsible of collapsibles) {
			if (collapsible.nodeId === nId) {
				if (operation === 'increment') collapsible.hideCount++
				else collapsible.hideCount--
				const targetIds = findTargets(store, nId)
				for (const targetId of targetIds) {
					recursiveTraverse(targetId)
				}
			}
		}
	}
}

function collapse(store: StoreType, nodeId: string) {
	const targetNodeIds = findTargets(store, nodeId)
	for (const targetNodeId of targetNodeIds) traverseAndIncrement(store, targetNodeId, 'increment')
}
function expand(store: StoreType, nodeId: string) {
	const targetNodeIds = findTargets(store, nodeId)
	for (const targetNodeId of targetNodeIds) traverseAndIncrement(store, targetNodeId, 'decrement')
}

export function toggleExpandAndCollapse(store: StoreType, nodeId: string) {
	const collapsibles = getCollapsibles(store, { nodeId: nodeId })
	if (collapsibles.length === 0) return // when the collapsible feature is disabled, there will be no collapbsible objects
	if (collapsibles.length > 1) throw 'there should only be one collapsible object per node'
	const collapsible = collapsibles[0]
	if (collapsible.state === 'expanded') collapse(store, nodeId)
	else expand(store, nodeId)
	store.collapsibleStore.update((arr) => {
		for (const c of arr) if (c.id === collapsible.id) c.toggleState()
		return [...arr]
	})
}

export function getCollapsibles(store: StoreType, filter?: { [key: string]: any }) {
	let collapsibles = Object.values(get(store.collapsibleStore))
	// filter the array of anchors for elements that match filter
	// Example: if filter = {sourceOrTarget: 'source', positionX: 35} then we will
	//return all anchors with sourceOrTarget = source AND poxitionX = 35
	if (filter !== undefined) {
		collapsibles = collapsibles.filter((collapsible) => {
			for (let filterKey in filter) {
				const filterValue = filter[filterKey]
				if (collapsible[filterKey as keyof CollapsibleType] !== filterValue) return false
			}
			return true
		})
	}
	// return list of anchors
	return collapsibles
}

/*
  This function is responsible for filtering nodes should be displayed based on Collapsible.
  It also filters node-associated elements such as anchors, edges, etc. so that when you collapse a node, the 
  edges also hide.
  There is a better way to implement this with foreign keys; when collapsing a node, you would also collapse any rows with a foreign key 
  linking to that node (like a cascading delete in SQL, but with hiding instead of deleting)
*/
export function filterByCollapsible(
	store: StoreType,
	nodes: NodeType[],
	resizeNodes: ResizeNodeType[],
	anchors: AnchorType[],
	edges: EdgeType[]
) {
	// filter nodes for the collapsible nodes feature
	const filteredNodes = nodes.filter((node) => {
		const nodeId = node.id
		const collapssibleObj = get(store.collapsibleStore).find((e) => e.nodeId === nodeId)
		if (collapssibleObj === undefined) return true
		return collapssibleObj.isHidden() === false
	})
	const filteredNodeIds = filteredNodes.map((e) => e.id)
	// filter resizeNodes
	const filteredResizeNodes = resizeNodes.filter((resizeNode) =>
		filteredNodeIds.includes(resizeNode.nodeId)
	)
	const filteredAnchors = anchors.filter((selfAnchor) => {
		const otherAnchorId = selfAnchor.getOtherAnchorId()
		const otherAnchor = get(store.anchorsStore)[otherAnchorId]

		if (filteredNodeIds.includes(selfAnchor.nodeId) && filteredNodeIds.includes(otherAnchor.nodeId))
			return true
		return false
	})
	const filteredEdgeIds = new Set(filteredAnchors.map((e) => e.edgeId))
	const filteredEdges = edges.filter((edge) => filteredEdgeIds.has(edge.id))

	return {
		filteredNodes,
		filteredResizeNodes,
		filteredAnchors,
		filteredEdges
	}
}
