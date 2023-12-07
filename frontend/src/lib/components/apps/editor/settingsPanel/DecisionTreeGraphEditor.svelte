<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { Network } from 'lucide-svelte'
	import type { AppComponent, DecisionTreeNode } from '../component'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Svelvet from '$lib/components/graph/svelvet/container/views/Svelvet.svelte'
	import type { UserEdgeType } from '$lib/components/graph/svelvet/types'
	import { NODE, type Node } from '$lib/components/graph'
	import DecisionTreeGraphNode from './DecisionTreeGraphNode.svelte'
	import { onMount } from 'svelte'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import { getNextId } from '$lib/components/flows/idUtils'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'

	export let component: AppComponent
	export let nodes: DecisionTreeNode[]

	let drawer: Drawer | undefined = undefined
	let displayedNodes: Node[] = []
	let edges: UserEdgeType[] = []
	let width: number, height: number
	let fullWidth: number
	let scroll = false
	let fullSize = false
	let selectedNodeId: string | null = null

	$: selectedNode = nodes?.find((node) => node.id == selectedNodeId)

	function layoutNodes(nodes: Node[]): { nodes: Node[]; height: number; width: number } {
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}

		if (nodes.length == 0) {
			return {
				nodes: [],
				height: 0,
				width: 0
			}
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
			nodes: dag.descendants().map((des) => ({
				...des.data,
				id: des.data.id,
				position: {
					x: des.x
						? des.data.loopDepth * 50 +
						  des.x +
						  (fullSize ? fullWidth : width) / 2 -
						  boxSize.width / 2 -
						  NODE.width / 2
						: 0,
					y: des.y || 0
				}
			})),
			height: boxSize.height + NODE.height,
			width: boxSize.width + NODE.width
		}
	}

	$: layered = layoutNodes(displayedNodes)

	$: console.log(displayedNodes)
	$: console.log(layered)

	let hfull = Math.max(layered.height, 400)
	fullWidth = layered.width
	height = fullSize ? hfull : Math.min(hfull, 1200 ?? window.innerHeight * 1.5)

	async function createGraph() {
		try {
			displayedNodes = []
			edges = []

			nodes?.forEach((graphNode, index) => {
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
								selected: selectedNodeId == graphNode.id
							},
							cb: (e: string, detail: any) => {
								if (e == 'select') {
									selectedNodeId = detail
								} else if (e == 'add') {
									const newNode = {
										id: 'd',
										label: 'd',
										next: []
									}

									graphNode.next.push(newNode)

									nodes.push(newNode)
									graphNode.next.push({
										id: newNode.id,
										condition: {
											type: 'eval',
											expr: 'true',
											fieldType: 'boolean'
										}
									})

									createGraph()
								} else if (e === 'node') {
									const nextId = getNextId(nodes.map((n) => n.id))

									const newNode = {
										id: nextId,
										label: nextId,
										next: []
									}

									nodes[index].next.push(newNode)

									nodes.push(newNode)

									createGraph()
								} else if (e === 'branch') {
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
					loopDepth: 0,
					childNodes: []
				})

				graphNode.next.forEach((nextNode) => {
					edges.push({
						id: `${graphNode.id}-${nextNode.id}`,
						source: graphNode.id,
						target: nextNode.id,
						label: ''
					})
				})
			})
		} catch (e) {
			console.error(e)
		}
	}

	onMount(async () => {
		await createGraph()
	})
</script>

<Drawer bind:this={drawer} on:close={() => {}} on:open={() => {}} size="1200px">
	<DrawerContent
		title="Decision tree"
		on:close={drawer.closeDrawer}
		noPadding
		tooltip="Decision tree graph editor"
	>
		<Splitpanes>
			<Pane size={50}>
				<Svelvet
					on:expand={() => {}}
					download={false}
					highlightEdges={false}
					locked
					dataflow={false}
					nodes={layered?.nodes}
					{edges}
					{height}
					{scroll}
					nodeSelected={false}
					background={false}
					width={fullWidth}
				/>
			</Pane>
			<Pane size={50}>
				<div class="h-full w-full bg-surface p-2">
					{#if selectedNode}
						{#each selectedNode.next as subNode}
							{#if subNode.condition}
								<InputsSpecEditor
									key={`Goes to ${subNode.id} if:`}
									bind:componentInput={subNode.condition}
									id={'sad'}
									userInputEnabled={false}
									shouldCapitalize={true}
									resourceOnly={false}
									fieldType={subNode.condition?.['fieldType']}
									subFieldType={subNode.condition?.['subFieldType']}
									format={subNode.condition?.['format']}
									selectOptions={subNode.condition?.['selectOptions']}
									tooltip={subNode.condition?.['tooltip']}
									fileUpload={subNode.condition?.['fileUpload']}
									placeholder={subNode.condition?.['placeholder']}
									customTitle={subNode.condition?.['customTitle']}
									displayType={false}
								/>
							{/if}
						{/each}
					{/if}
				</div>
			</Pane>
		</Splitpanes>
	</DrawerContent>
</Drawer>

<div class="p-2">
	<Button
		tooltip="Decision tree graph editor"
		on:click={() => {
			drawer?.openDrawer()
		}}
		size="xs"
		color="light"
		startIcon={{ icon: Network }}
	>
		Graph editor
	</Button>
</div>
