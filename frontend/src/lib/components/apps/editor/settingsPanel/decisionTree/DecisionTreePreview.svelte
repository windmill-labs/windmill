<script lang="ts">
	import { NODE } from '$lib/components/graph'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import {
		SvelteFlow,
		type Node,
		type Edge,
		ConnectionLineType,
		type Viewport,
		Controls
	} from '@xyflow/svelte'

	import { addNewBranch, addNode, getParents, insertNode, removeBranch, removeNode } from './utils'

	import DecisionTreeGraphNode from '../DecisionTreeGraphNode.svelte'
	import DecisionTreeGraphHeader from '../DecisionTreeGraphHeader.svelte'

	import { writable, type Writable } from 'svelte/store'
	import type { AppComponent, DecisionTreeNode } from '../../component'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { deleteGridItem } from '../../appUtils'

	export let nodes: DecisionTreeNode[]
	export let paneWidth = 0
	export let paneHeight = 0
	export let component: AppComponent

	const dispatch = createEventDispatcher()

	const { selectedNodeId } = getContext<{
		selectedNodeId: Writable<string | undefined>
	}>('DecisionTreeEditor')

	const { app, runnableComponents, componentControl, debuggingComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	function addSubGrid() {
		const numberOfPanes = nodes.length
		if (!$app.subgrids) {
			$app.subgrids = {}
		}
		$app.subgrids[`${component.id}-${numberOfPanes}`] = []

		component.numberOfSubgrids = nodes.length + 1
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
		graphNode: DecisionTreeNode | undefined,
		parentIds: string[],
		branchInsert: boolean = false
	) {
		switch (event) {
			case 'select':
				$selectedNodeId = detail
				const index = nodes.findIndex((node) => node.id === detail)
				$componentControl?.[component.id]?.setTab?.(index)
				$debuggingComponents[component.id] = index

				break
			case 'nodeInsert': {
				addSubGrid()

				if (branchInsert) {
					if (parentIds.length === 1 && graphNode) {
						// console.log('A', parentIds)
						nodes = insertNode(nodes, parentIds[0], graphNode)
					} else {
						// console.log('B', parentIds)
						// find parent with multiple next
						const parentWithMultipleNext = nodes.find((node) => {
							return node.next.length > 1 && parentIds.includes(node.id)
						})

						if (!parentWithMultipleNext) {
							deleteSubgrid(nodes.length - 1)
							return nodes
						}

						nodes = insertNode(nodes, parentWithMultipleNext.id, graphNode!)
					}
				} else {
					nodes = addNode(nodes, graphNode)
				}

				break
			}

			case 'delete': {
				const graphNodeIndex = nodes.findIndex((node) => node.id == graphNode?.id)

				if (graphNodeIndex > -1) {
					deleteSubgrid(graphNodeIndex)
				}

				nodes = removeNode(nodes, graphNode)

				$debuggingComponents = Object.fromEntries(
					Object.entries($debuggingComponents).filter(([key]) => key !== component.id)
				)

				break
			}
			case 'addBranch': {
				addSubGrid()
				nodes = addNewBranch(nodes, graphNode!)
				break
			}
			case 'removeBranch': {
				nodes = removeBranch(nodes, graphNode, parentIds[0], (nodeId) => {
					const index = nodes.findIndex((node) => node.id === nodeId)

					deleteSubgrid(index)
				})
				break
			}
			default:
				break
		}
		dispatch('render')
	}

	function graphBuilder(decisionTreeNodes: DecisionTreeNode[]): {
		edges: Edge[]
		nodes: Node[]
	} {
		const nodes: Node[] = []
		const edges: Edge[] = []

		function addNode(
			node: DecisionTreeNode,
			type: string = 'step',
			data: {
				canDelete: boolean
				canAddBranch: boolean
				selected: boolean
				index: number
			}
		) {
			nodes.push({
				id: node.id,
				type,
				position: { x: -1, y: -1 },
				data: {
					node,
					nodeCallbackHandler,
					...data
				}
			})
		}

		const parents: { [key: string]: string[] } = {}

		function addEdge(source: string, target: string) {
			parents[target] = [...(parents[target] ?? []), source]

			edges.push({
				id: `${source}-${target}`,
				source,
				target,
				type: 'edge'
			})
		}

		function processNodes(decisionTreeNodes: DecisionTreeNode[], beforeNode: Node, nextNode: Node) {
			if (decisionTreeNodes.length === 0) {
				addEdge(beforeNode.id, nextNode.id)
			} else {
				decisionTreeNodes.forEach((node, index) => {
					const cannotAddBranch =
						node.next.length === 0 ||
						(node.next.length === 1 && getParents(decisionTreeNodes, node.next[0].id).length > 1)
					addNode(node, 'step', {
						canDelete: true,
						canAddBranch: !cannotAddBranch,
						selected: false,
						index
					})

					if (node.next.length === 1) {
						addEdge(node.id, node.next[0].id)
					} else if (node.next.length > 0) {
						node.next.forEach((next, innerIndex) => {
							const branchHeaderId = `${node.id}-branch-${innerIndex}`
							const header = {
								id: branchHeaderId,
								type: 'start',
								position: { x: -1, y: -1 },
								data: {
									node: {
										label: innerIndex === 0 ? 'Default branch' : `Branch ${innerIndex}`,
										id: branchHeaderId,
										allowed: undefined,
										next: []
									},
									canDelete: false,
									nodeCallbackHandler,
									realNode: next,
									branchHeader: true
								}
							}

							nodes.push(header)
							addEdge(node.id, header.id)
							addEdge(header.id, next.id)
						})
					}

					if (index === 0) {
						addEdge(beforeNode.id, node.id)
					}
				})
			}
		}

		const startNode = {
			id: 'start',
			type: 'start',
			position: { x: -1, y: -1 },
			data: {
				node: {
					id: 'start',
					label: 'Start',
					allowed: undefined,
					next: []
				},
				canDelete: false,
				nodeCallbackHandler
			}
		}

		const endNode = {
			id: 'end',
			type: 'end',
			position: { x: -1, y: -1 },
			data: {
				node: {
					id: 'end',
					label: 'End',
					allowed: undefined,
					next: []
				},
				canDelete: false,
				nodeCallbackHandler
			}
		}

		const lastNodesIds = decisionTreeNodes
			.filter((node) => {
				return node.next.length == 0
			})
			.map((node) => node.id)

		lastNodesIds.forEach((id) => {
			addEdge(id, endNode.id)
		})

		nodes.push(startNode)
		nodes.push(endNode)

		processNodes(decisionTreeNodes, startNode, endNode)

		Object.keys(parents).forEach((key) => {
			const node = nodes.find((n) => n.id === key)

			if (node) {
				node.data.parentIds = parents[key]
			}
		})

		return {
			edges,
			nodes
		}
	}

	$: graph = graphBuilder(nodes)

	const nodesStore = writable<Node[]>([])
	const edgesStore = writable<Edge[]>([])

	function layoutNodes(nodes: Node[]): Node[] {
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
		})) as any

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
				x: des.x ? des.x - boxSize.width / 2 - NODE.width / 2 : 0,
				y: des.y || 0
			}
		}))

		return newNodes
	}

	function updateStores() {
		$nodesStore = layoutNodes(graph?.nodes)
		$edgesStore = graph.edges
	}

	$: graph && updateStores()

	const viewport = writable<Viewport>({
		x: 0,
		y: 5,
		zoom: 1
	})

	function centerViewport(width: number) {
		viewport.update((vp) => ({
			...vp,
			x: width / 2,
			y: vp.y
		}))
	}

	$: paneWidth && centerViewport(paneWidth)

	const nodeTypes = {
		step: DecisionTreeGraphNode,
		start: DecisionTreeGraphHeader,
		end: DecisionTreeGraphHeader
	} as any
</script>

<SvelteFlow
	nodes={nodesStore}
	edges={edgesStore}
	{nodeTypes}
	{viewport}
	height={paneHeight}
	minZoom={0.5}
	connectionLineType={ConnectionLineType.SmoothStep}
	defaultEdgeOptions={{ type: 'smoothstep' }}
	zoomOnDoubleClick={false}
	elementsSelectable={false}
	proOptions={{ hideAttribution: true }}
	nodesDraggable={false}
>
	<Controls position="top-right" orientation="horizontal" showLock={false} />
</SvelteFlow>
