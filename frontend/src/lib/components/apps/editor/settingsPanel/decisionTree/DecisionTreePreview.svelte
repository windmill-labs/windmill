<script lang="ts">
	import Svelvet from '$lib/components/graph/svelvet/container/views/Svelvet.svelte'
	import type { UserEdgeType } from '$lib/components/graph/svelvet/types'
	import { NODE, type Node } from '$lib/components/graph'
	import { createEventDispatcher, getContext, onMount } from 'svelte'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import { deepEqual } from 'fast-equals'

	import DecisionTreeGraphNode from '../DecisionTreeGraphNode.svelte'
	import DecisionTreeGraphHeader from '../DecisionTreeGraphHeader.svelte'

	import type { Writable } from 'svelte/store'
	import type { AppComponent, DecisionTreeNode } from '../../component'
	import {
		addBranch,
		addNewBranch,
		addNode,
		getParents,
		insertFirstNode,
		removeBranch,
		removeNode
	} from './utils'
	import { createEdge, createNode } from './nodeHelpers'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { deleteGridItem } from '../../appUtils'

	export let nodes: DecisionTreeNode[]
	export let rebuildOnChange: any = undefined
	export let paneWidth = 0
	export let paneHeight = 0
	export let component: AppComponent

	let displayedNodes: Node[] = []
	let edges: UserEdgeType[] = []
	let scroll = false

	const dispatch = createEventDispatcher()

	const { selectedNodeId } = getContext<{
		selectedNodeId: Writable<string | undefined>
	}>('DecisionTreeEditor')

	$: rebuildOnChange && triggerRebuild()

	let oldRebuildOnChange = rebuildOnChange ? JSON.parse(JSON.stringify(rebuildOnChange)) : undefined

	function triggerRebuild() {
		if (!deepEqual(oldRebuildOnChange, rebuildOnChange)) {
			oldRebuildOnChange = JSON.parse(JSON.stringify(rebuildOnChange))
			createGraph()
		}
	}

	function buildStartNode() {
		const startNodeConfig = {
			id: 'start',
			data: {
				custom: {
					component: DecisionTreeGraphNode,
					props: {
						node: {
							id: 'start',
							label: 'Start',
							next: {
								id: nodes[0].id,
								condition: {
									type: 'evalv2',
									expr: 'true',
									fieldType: 'boolean'
								}
							}
						},
						canDelete: false,
						isHead: true
					},
					cb: (e: string, detail: any) => nodeCallbackHandler(e, detail, nodes[0], [])
				}
			}
		}

		const startNode = createNode(startNodeConfig)
		displayedNodes.push(startNode)
		edges.push(
			createEdge({
				id: `start-${nodes[0].id}`,
				source: 'start',
				target: nodes[0].id
			})
		)
	}
	const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	function buildEndNode() {
		const lastNodesIds = nodes
			.filter((node) => {
				return node.next.length == 0
			})
			.map((node) => node.id)

		displayedNodes.push(
			createNode({
				id: 'end',
				data: {
					custom: {
						component: DecisionTreeGraphNode,
						props: {
							node: {
								id: 'end',
								label: 'End',
								next: []
							},
							canDelete: false,
							isTail: true
						},
						cb: (e: string, detail: any) => {
							if (e == 'select') {
								$selectedNodeId = detail
							}
						}
					}
				},
				parentIds: lastNodesIds
			})
		)

		lastNodesIds.forEach((lastNodeId) => {
			edges.push(
				createEdge({
					id: `${lastNodeId}-end`,
					source: lastNodeId,
					target: 'end'
				})
			)
		})
	}

	function addSubGrid(addTwo: boolean = false) {
		const numberOfPanes = nodes.length
		if (!$app.subgrids) {
			$app.subgrids = {}
		}
		$app.subgrids[`${component.id}-${numberOfPanes}`] = []

		if (addTwo) {
			$app.subgrids[`${component.id}-${numberOfPanes + 1}`] = []
		}
		component.numberOfSubgrids = nodes.length + (addTwo ? 2 : 1)
	}

	function deleteSubgrid(index: number) {
		let subgrid = `${component.id}-${index}`

		if (!$app.subgrids![subgrid]) {
			return
		}

		for (const item of $app!.subgrids![subgrid]) {
			const components = deleteGridItem($app, item.data, subgrid)
			for (const key in components) {
				delete $runnableComponents[key]
			}
		}
		$runnableComponents = $runnableComponents
		for (let i = index; i < nodes.length - 1; i++) {
			$app!.subgrids![`${component.id}-${i}`] = $app!.subgrids![`${component.id}-${i + 1}`]
		}
		nodes.splice(index, 1)
		delete $app!.subgrids![`${component.id}-${nodes.length}`]

		nodes = nodes
		component.numberOfSubgrids = nodes.length
		$app = $app
	}

	function nodeCallbackHandler(
		event: string,
		detail: string,
		graphNode: DecisionTreeNode,
		parentIds
	) {
		switch (event) {
			case 'select':
				$selectedNodeId = detail
				break
			case 'nodeInsert':
				addSubGrid()
				nodes = addNode(nodes, graphNode)
				break
			case 'branchInsert': {
				addSubGrid(true)

				nodes = addBranch(nodes, graphNode)
				break
			}
			case 'delete': {
				const graphhNodeIndex = nodes.findIndex((node) => node.id == graphNode.id)
				if (graphhNodeIndex > -1) {
					deleteSubgrid(graphhNodeIndex)
				}
				nodes = removeNode(nodes, graphNode)
				break
			}
			case 'addBranch': {
				addSubGrid()
				nodes = addNewBranch(nodes, graphNode)
				break
			}
			case 'removeBranch': {
				nodes = removeBranch(nodes, graphNode, parentIds[0], (nodeId) => {
					const index = nodes.findIndex((node) => node.id === nodeId)
					deleteSubgrid(index)
				})
				break
			}
			case 'firstNodeInsert':
				addSubGrid()
				nodes = insertFirstNode(nodes)
				break
			default:
				break
		}
		dispatch('render')
	}

	function buildGraphNodes() {
		nodes?.forEach((graphNode) => {
			const parentIds = getParents(nodes, graphNode)
			const parentNext = nodes.find((node) => node.id == parentIds[0])?.next
			const hasParentBranches = parentNext ? parentNext.length > 1 : false

			if (hasParentBranches) {
				const branchHeaderId = `${parentIds[0]}-${graphNode.id}-branch-header`

				displayedNodes.push(
					createNode({
						id: branchHeaderId,
						data: {
							custom: {
								component: DecisionTreeGraphHeader,
								props: {
									node: graphNode,
									canDelete: true,
									isHead: true
								},
								cb: (e: string, detail: any) => nodeCallbackHandler(e, detail, graphNode, parentIds)
							}
						},
						parentIds: parentIds
					})
				)

				displayedNodes.push(
					createNode({
						id: graphNode.id,
						data: {
							custom: {
								component: DecisionTreeGraphNode,
								props: {
									node: graphNode,
									canDelete: graphNode.next.length === 1,
									isHead: graphNode.next.length === 0
								},
								cb: (e: string, detail: any) => nodeCallbackHandler(e, detail, graphNode, parentIds)
							}
						},
						parentIds: [branchHeaderId]
					})
				)

				edges.push(
					createEdge({
						id: `${graphNode.id}-${branchHeaderId}`,
						source: branchHeaderId,
						target: graphNode.id
					})
				)

				graphNode.next.forEach((nextNode) => {
					edges.push(
						createEdge({
							id: `${graphNode.id}-${nextNode.id}`,
							source: graphNode.id,
							target: nextNode.id
						})
					)
				})
			} else {
				displayedNodes.push(
					createNode({
						id: graphNode.id,
						data: {
							custom: {
								component: DecisionTreeGraphNode,
								props: {
									node: graphNode,
									canDelete: graphNode.next.length === 1,
									isHead: graphNode.next.length === 0
								},
								cb: (e: string, detail: any) => nodeCallbackHandler(e, detail, graphNode, parentIds)
							}
						},
						parentIds: parentIds
					})
				)

				// if node has multiple next, it means it needs to be connected to a branch header
				const hasMultipleNext = graphNode.next.length > 1

				graphNode.next.forEach((nextNode) => {
					const target = hasMultipleNext
						? `${graphNode.id}-${nextNode.id}-branch-header`
						: nextNode.id

					edges.push(
						createEdge({
							id: `${graphNode.id}-${nextNode.id}`,
							source: graphNode.id,
							target
						})
					)
				})
			}
		})
	}

	function layoutNodes(nodes: Node[]): { nodes: Node[]; height: number; width: number } {
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}
		const stratify = dagStratify().id(({ id }: Node) => id)
		const dag = stratify(nodes)

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

		return {
			nodes: dag.descendants().map((des) => {
				return {
					...des.data,
					id: des.data.id,
					position: {
						x: des.x
							? des.data.loopDepth * 50 + des.x + paneWidth / 2 - boxSize.width / 2 - NODE.width / 2
							: 0,
						y: des.y || 0
					}
				}
			}),
			height: boxSize.height + NODE.height,
			width: boxSize.width + NODE.width
		}
	}

	function resetGraphData() {
		displayedNodes = []
		edges = []
	}

	function createGraph() {
		try {
			resetGraphData()
			buildStartNode()
			buildGraphNodes()
			buildEndNode()
			applyLayoutToNodes()
		} catch (e) {
			console.error(e)
		}
	}

	function applyLayoutToNodes() {
		const layered = layoutNodes(displayedNodes)
		displayedNodes = layered.nodes
	}

	let mounted = false

	onMount(() => {
		createGraph()
		mounted = true
	})
</script>

{#if mounted}
	<Svelvet
		download={false}
		highlightEdges={false}
		locked
		dataflow={false}
		nodes={displayedNodes}
		{edges}
		height={paneHeight}
		{scroll}
		nodeSelected={false}
		background={false}
		width={paneWidth}
	/>
{/if}
