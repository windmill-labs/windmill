<script lang="ts">
	import { NODE } from '$lib/components/graph'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import {
		SvelteFlow,
		type Node,
		type Edge,
		ConnectionLineType,
		Controls,
		SvelteFlowProvider
	} from '@xyflow/svelte'

	import {
		addNewBranch,
		addNode,
		getFirstNode,
		getParents,
		insertNode,
		removeBranch,
		removeNode
	} from './utils'

	import DecisionTreeGraphNode from '../DecisionTreeGraphNode.svelte'
	import DecisionTreeGraphHeader from '../DecisionTreeGraphHeader.svelte'

	import { type Writable } from 'svelte/store'
	import type { AppComponent, DecisionTreeNode } from '../../component'
	import { getContext, untrack } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { deleteGridItem } from '../../appUtils'

	interface Props {
		nodes: DecisionTreeNode[]
		paneWidth?: number
		paneHeight?: number
		component: AppComponent
		onrender: () => void
	}

	let {
		nodes = $bindable(),
		paneWidth = 0,
		paneHeight = 0,
		component = $bindable(),
		onrender
	}: Props = $props()

	let nodesStore = $state.raw<Node[]>([])
	let edgesStore = $state.raw<Edge[]>([])

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
		onrender()
	}

	function graphBuilder(decisionTreeNodes: DecisionTreeNode[]) {
		const nodes: Node[] = []
		const edges: Edge[] = []

		function addNode(
			node: DecisionTreeNode,
			type: string = 'step',
			data: {
				canDelete: boolean
				canAddBranch: boolean
				index: number
				parentIds?: string[]
			},
			x = nodeCallbackHandler
		) {
			nodes.push({
				id: node.id,
				type,
				position: { x: -1, y: -1 },
				data: {
					node,
					nodeCallbackHandler: x,
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

		function processNodes(decisionTreeNodes: DecisionTreeNode[]) {
			decisionTreeNodes.forEach((graphNode, index) => {
				const parentIds = getParents(decisionTreeNodes, graphNode.id)
				const parentNext = decisionTreeNodes.find((node) => node.id == parentIds[0])?.next
				const hasParentBranches = parentNext ? parentNext.length > 1 : false

				if (hasParentBranches) {
					const positionRelativeToParent = parentNext?.findIndex((next) => next.id == graphNode.id)
					const branchHeaderId = `${parentIds[0]}-${graphNode.id}-branch-header`

					// We create a header node for the branch, which will be the parent of the actual node
					const header = {
						id: branchHeaderId,
						type: 'start',
						position: { x: -1, y: -1 },
						data: {
							node: {
								label:
									positionRelativeToParent === 0
										? 'Default branch'
										: `Branch ${positionRelativeToParent}`,
								id: branchHeaderId,
								allowed: undefined,
								next: [],
								parentIds: [parentIds[0]]
							},
							canDelete: true,
							nodeCallbackHandler: (e, d) => {
								nodeCallbackHandler(e, d, graphNode, parentIds, true)
							},
							branchHeader: true
						}
					}

					nodes.push(header)

					const cannotAddBranch =
						graphNode.next.length === 0 ||
						(graphNode.next.length === 1 &&
							getParents(decisionTreeNodes, graphNode.next[0].id).length > 1)

					// We create the actual node

					addNode(
						graphNode,
						'step',
						{
							canDelete: !(graphNode.next.length > 1 && parentIds.length > 1),
							canAddBranch: !cannotAddBranch,
							index,
							parentIds: [
								branchHeaderId,
								...parentIds.filter((pId) => {
									const firstLetter = branchHeaderId.split('-')[0]
									return pId !== firstLetter
								})
							]
						},
						(e, d) => {
							nodeCallbackHandler(e, d, graphNode, parentIds, false)
							return undefined
						}
					)

					addEdge(branchHeaderId, graphNode.id)

					if (graphNode.next.length === 1) {
						addEdge(graphNode.id, graphNode.next[0].id)
					} else {
						graphNode.next.forEach((nextNode) => {
							addEdge(graphNode.id, `${graphNode.id}-${nextNode.id}-branch-header`)
						})
					}
				} else {
					const cannotAddBranch =
						graphNode.next.length === 0 ||
						(graphNode.next.length === 1 &&
							getParents(decisionTreeNodes, graphNode.next[0].id).length > 1)

					addNode(
						graphNode,
						'step',
						{
							canDelete:
								!cannotAddBranch && (graphNode.next.length == 1 || !parentIds.includes('start')),
							canAddBranch: !cannotAddBranch,
							index,
							parentIds: parentIds
						},
						(e, d) => {
							nodeCallbackHandler(e, d, graphNode, parentIds, false)
							return undefined
						}
					)

					// if node has multiple next, it means it needs to be connected to a branch header
					const hasMultipleNext = graphNode.next.length > 1

					graphNode.next.forEach((nextNode) => {
						const target = hasMultipleNext
							? `${graphNode.id}-${nextNode.id}-branch-header`
							: nextNode.id

						addEdge(graphNode.id, target)
					})
				}
			})
		}

		const firstNode = getFirstNode(decisionTreeNodes)

		if (firstNode) {
			const startNode = {
				id: 'start',
				type: 'start',
				position: { x: -1, y: -1 },
				data: {
					node: {
						id: 'start',
						label: 'Start',
						allowed: undefined,
						next: [
							{
								id: firstNode.id,
								condition: {
									type: 'evalv2',
									expr: 'true',
									fieldType: 'boolean'
								}
							}
						]
					},
					canDelete: false,
					nodeCallbackHandler
				}
			}
			nodes.push(startNode)
			addEdge('start', firstNode.id)
		}

		processNodes(decisionTreeNodes)

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

		nodes.push(endNode)

		Object.keys(parents).forEach((key) => {
			const node = nodes.find((n) => n.id === key)

			if (node) {
				node.data.parentIds = parents[key]
			}
		})

		nodesStore = layoutNodes(nodes)
		edgesStore = edges
	}

	$effect(() => {
		;[nodes]
		untrack(() => graphBuilder(nodes))
	})

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

	const nodeTypes = {
		step: DecisionTreeGraphNode,
		start: DecisionTreeGraphHeader,
		end: DecisionTreeGraphHeader
	} as any
</script>

<SvelteFlowProvider>
	<SvelteFlow
		nodes={nodesStore}
		edges={edgesStore}
		{nodeTypes}
		height={paneHeight}
		minZoom={0.5}
		initialViewport={{
			x: paneWidth / 2,
			y: 5,
			zoom: 1
		}}
		connectionLineType={ConnectionLineType.SmoothStep}
		defaultEdgeOptions={{ type: 'smoothstep' }}
		zoomOnDoubleClick={false}
		elementsSelectable={false}
		proOptions={{ hideAttribution: true }}
		nodesDraggable={false}
	>
		<div class="absolute inset-0 !bg-surface-secondary"></div>

		<Controls position="top-right" orientation="horizontal" showLock={false} />
	</SvelteFlow>
</SvelteFlowProvider>
