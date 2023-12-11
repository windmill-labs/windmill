<script lang="ts">
	import Svelvet from '$lib/components/graph/svelvet/container/views/Svelvet.svelte'
	import type { UserEdgeType } from '$lib/components/graph/svelvet/types'
	import { NODE, type Node } from '$lib/components/graph'
	import { createEventDispatcher, getContext, onMount } from 'svelte'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import { deepEqual } from 'fast-equals'

	import DecisionTreeGraphNode from '../DecisionTreeGraphNode.svelte'

	import type { Writable } from 'svelte/store'
	import type { DecisionTreeNode } from '../../component'
	import { Alert } from '$lib/components/common'
	import { addBranch, addNode, insertFirstNode, removeNode } from './utils'

	export let nodes: DecisionTreeNode[]
	export let minHeight: number = 0
	export let rebuildOnChange: any = undefined
	export let paneWidth = 0
	export let paneHeight = 0
	export let editable: boolean = true

	let displayedNodes: Node[] = []
	let edges: UserEdgeType[] = []
	let fullWidth: number = 0
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
							? des.data.loopDepth * 50 +
							  des.x +
							  fullWidth / 2 -
							  boxSize.width / 2 -
							  NODE.width / 2 +
							  300
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

	async function createGraph() {
		try {
			displayedNodes = []
			edges = []

			displayedNodes.push({
				type: 'node',
				id: 'start',
				position: {
					x: -1,
					y: -1
				},
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
							isHead: true,
							canDelete: false
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
				},
				width: NODE.width,
				height: NODE.height,
				borderColor: '#999',
				sourcePosition: 'bottom',
				targetPosition: 'top',
				parentIds: [],
				loopDepth: 0
			})

			edges.push({
				id: `start-${nodes[0].id}`,
				source: 'start',
				target: nodes[0].id,
				label: '',
				edgeColor: '#999'
			})

			const nextMap: { [key: string]: string[] } = {}

			nodes?.forEach((graphNode, index) => {
				const parentIds = computeParentIds(nodes, graphNode)

				nextMap[graphNode.id] = graphNode.next.map((nextNode) => nextNode.id)

				/*

				
				const hasSiblings = parentIds.length === 1 && nextMap[parentIds[0]].length > 1
				*/

				displayedNodes.push({
					type: 'node',
					id: graphNode.id,
					position: {
						x: -1,
						y: -1
					},
					data: {
						custom: {
							component: DecisionTreeGraphNode,
							props: {
								node: graphNode,
								editable: editable,
								canDelete: graphNode.next.length === 1
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
								}
							}
						}
					},
					width: NODE.width,
					height: NODE.height,
					borderColor: '#999',
					sourcePosition: 'bottom',
					targetPosition: 'top',
					parentIds: parentIds,
					loopDepth: 0
				})

				graphNode.next.forEach((nextNode) => {
					edges.push({
						id: `${graphNode.id}-${nextNode.id}`,
						source: graphNode.id,
						target: nextNode.id,
						label: '',
						edgeColor: '#999'
					})
				})
			})

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
							canDelete: false
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
			fullWidth = layered.width
		} catch (e) {
			console.error(e)
		}
	}

	onMount(async () => {
		await createGraph()
	})
</script>

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
	width={paneWidth - 1}
/>
{#if !editable}
	<Alert type="info" title="Debug nodes" size="xs">Click on a node to debug its content.</Alert>
{/if}
