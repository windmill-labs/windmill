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
	import type { DecisionTreeNode } from '../../component'
	import { Alert } from '$lib/components/common'
	import {
		addBranch,
		addNewBranch,
		addNode,
		insertFirstNode,
		removeBranch,
		removeNode
	} from './utils'
	import { createEdge, createNode } from './nodeHelpers'

	export let nodes: DecisionTreeNode[]
	export let rebuildOnChange: any = undefined
	export let paneWidth = 0
	export let paneHeight = 0
	export let editable: boolean = true

	let displayedNodes: Node[] = []
	let edges: UserEdgeType[] = []
	let scroll = false

	const dispatch = createEventDispatcher()

	const { selectedNodeId } = getContext<{
		selectedNodeId: Writable<string | undefined>
	}>('DecisionTreeEditor')

	$: {
		createGraph()
	}

	$: rebuildOnChange && triggerRebuild()

	let oldRebuildOnChange = rebuildOnChange ? JSON.parse(JSON.stringify(rebuildOnChange)) : undefined

	function triggerRebuild() {
		if (!deepEqual(oldRebuildOnChange, rebuildOnChange)) {
			oldRebuildOnChange = JSON.parse(JSON.stringify(rebuildOnChange))
			createGraph()
		}
	}

	// Create a start node to connect the first node.
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
					cb: (e: string, detail: any) => {
						if (e == 'select') {
							$selectedNodeId = detail
						} else if (e === 'nodeInsert') {
							nodes = insertFirstNode(nodes)
							dispatch('render')
						}
					}
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
				nodes = addNode(nodes, graphNode)
				break
			case 'branchInsert':
				nodes = addBranch(nodes, graphNode)
				break
			case 'delete':
				nodes = removeNode(nodes, graphNode)
				break
			case 'addBranch':
				nodes = addNewBranch(nodes, graphNode)
				break
			case 'removeBranch':
				nodes = removeBranch(nodes, graphNode, parentIds[0])

				break
			default:
				break
		}
		dispatch('render')
	}

	function buildGraphNodes() {
		nodes?.forEach((graphNode, index) => {
			const parentIds = computeParentIds(nodes, graphNode)
			const parentNext = nodes.find((node) => node.id == parentIds[0])?.next
			const hasParentBranches = parentNext ? parentNext.length > 1 : false

			if (hasParentBranches) {
				const branchHeaderId = `${parentIds[0]}-${graphNode.id}-branch-header`

				const branchHeaderNode = createNode({
					id: branchHeaderId,
					data: {
						custom: {
							component: DecisionTreeGraphHeader,
							props: {
								node: graphNode,
								editable: editable,
								canDelete: true,
								isHead: true
							},
							cb: (e: string, detail: any) => nodeCallbackHandler(e, detail, graphNode, parentIds)
						}
					},
					parentIds: parentIds
				})

				displayedNodes.push(branchHeaderNode)

				const displayedNode = createNode({
					id: graphNode.id,
					data: {
						custom: {
							component: DecisionTreeGraphNode,
							props: {
								node: graphNode,
								editable: editable,
								canDelete: graphNode.next.length === 1,
								isHead: graphNode.next.length === 0
							},
							cb: (e: string, detail: any) => {
								if (e == 'select') {
									$selectedNodeId = detail
								} else if (e == 'nodeInsert') {
									nodes = addNode(nodes, graphNode)
									dispatch('render')
								} else if (e === 'branchInsert') {
									nodes = addBranch(nodes, graphNode)
									dispatch('render')
								} else if (e === 'delete') {
									nodes = removeNode(nodes, graphNode)

									dispatch('render')
								} else if (e === 'addBranch') {
									nodes = addNewBranch(nodes, graphNode)
									dispatch('render')
								}
							}
						}
					},
					parentIds: [branchHeaderId]
				})

				displayedNodes.push(displayedNode)

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
									editable: editable,
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

	function computeParentIds(nodes: DecisionTreeNode[], node: DecisionTreeNode): string[] {
		let parentIds: string[] = []

		nodes.forEach((n) => {
			n.next.forEach((nextNode) => {
				if (nextNode.id == node.id) {
					parentIds.push(n.id)
				}
			})
		})

		// if first node, add start node as parent
		if (node.id == nodes[0].id) {
			parentIds.push('start')
		}

		return parentIds
	}

	function resetGraphData() {
		displayedNodes = []
		edges = []
	}

	async function createGraph() {
		try {
			resetGraphData()
			buildStartNode()
			buildGraphNodes()

			const lastNodesIds = nodes
				.filter((node) => {
					return node.next.length == 0
				})
				.map((node) => node.id)

			displayedNodes.push({
				type: 'node',
				id: 'end',
				position: {
					x: -1,
					y: -1
				},
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
							isHead: true
						},
						cb: (e: string, detail: any) => {
							if (e == 'select') {
								$selectedNodeId = detail
							}
						}
					}
				},
				width: NODE.width,
				height: NODE.height,
				borderColor: '#999',
				sourcePosition: 'bottom',
				targetPosition: 'top',
				parentIds: lastNodesIds,
				loopDepth: 0
			})

			lastNodesIds.forEach((lastNodeId) => {
				edges.push({
					id: `${lastNodeId}-end`,
					source: lastNodeId,
					target: 'end',
					label: '',
					edgeColor: '#999'
				})
			})

			const layered = layoutNodes(displayedNodes)

			displayedNodes = layered.nodes
		} catch (e) {
			console.error(e)
		}
	}

	let mounted = false

	onMount(async () => {
		await createGraph()
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
{#if !editable}
	<Alert type="info" title="Debug nodes" size="xs">Click on a node to debug its content.</Alert>
{/if}
