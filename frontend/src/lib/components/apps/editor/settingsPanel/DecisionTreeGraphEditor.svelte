<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { Network, X } from 'lucide-svelte'
	import type { AppComponent, DecisionTreeNode } from '../component'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Svelvet from '$lib/components/graph/svelvet/container/views/Svelvet.svelte'
	import type { UserEdgeType } from '$lib/components/graph/svelvet/types'
	import { NODE, type Node } from '$lib/components/graph'
	import DecisionTreeGraphNode from './DecisionTreeGraphNode.svelte'
	import { onMount, setContext } from 'svelte'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import { getNextId } from '$lib/components/flows/idUtils'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import { deepEqual } from 'fast-equals'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Section from '$lib/components/Section.svelte'
	import { writable } from 'svelte/store'

	export let component: AppComponent
	export let nodes: DecisionTreeNode[]
	export let minHeight: number = 0
	export let maxHeight: number | undefined = undefined
	export let rebuildOnChange: any = undefined

	let drawer: Drawer | undefined = undefined
	let displayedNodes: Node[] = []
	let edges: UserEdgeType[] = []
	let width: number, height: number
	let fullWidth: number = 0
	let scroll = false
	let fullSize = false

	$: {
		width && height && minHeight && selectedNodeId && createGraph()
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
							  200
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

		return parentIds
	}

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
								node: graphNode
							},
							cb: (e: string, detail: any) => {
								if (e == 'select') {
									$selectedNodeId = detail
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
					parentIds: computeParentIds(nodes, graphNode),
					loopDepth: 2
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

			const layered = layoutNodes(displayedNodes)

			displayedNodes = layered.nodes
			let hfull = Math.max(layered.height, minHeight)
			fullWidth = layered.width
			height = fullSize ? hfull : Math.min(hfull, maxHeight ?? window.innerHeight * 1.5)
		} catch (e) {
			console.error(e)
		}
	}

	onMount(async () => {
		await createGraph()
	})

	let paneWidth = 0
	let paneHeight = 0

	const selectedNodeId = writable<string | undefined>(undefined)

	$: selectedNode = nodes?.find((node) => node.id == $selectedNodeId)

	setContext('DecisionTreeEditor', { selectedNodeId })
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
				<div class="w-full h-full" bind:clientWidth={paneWidth} bind:clientHeight={paneHeight}>
					<Svelvet
						download={false}
						highlightEdges={true}
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
				</div>
			</Pane>
			<Pane size={50}>
				<div class="h-full w-full bg-surface p-4 flex flex-col gap-6">
					{#if selectedNode}
						<Section label="Conditions" class="w-full">
							{#if Array.isArray(selectedNode.next) && selectedNode.next.length === 1}
								<Alert type="info" title="This node has only one next node">
									You can add a new node by clicking on the "Add step" button in the top right
									corner of the node.
								</Alert>
							{:else}
								{#each selectedNode.next as subNode}
									{#if subNode.condition}
										<div class="flex flex-row gap-4 items-center w-full justify-center">
											<div class="grow">
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
											</div>
											<Button
												size="xs"
												color="red"
												startIcon={{ icon: X }}
												variant="border"
												iconOnly
											/>
										</div>
									{/if}
								{/each}
							{/if}

							<div class="flex flex-row gap-2 mt-4">
								<Button size="xs" color="light" startIcon={{ icon: Network }} variant="border">
									Add branch
								</Button>
							</div>
						</Section>

						<Section label="Actions">
							<div class="flex flex-row gap-2">
								<Button size="xs" color="light" startIcon={{ icon: Network }} variant="border"
									>Add node</Button
								>

								<Button size="xs" color="light" startIcon={{ icon: Network }} variant="border"
									>Add parent node</Button
								>
							</div>
						</Section>
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
