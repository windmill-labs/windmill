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

	$: console.log(nodes)

	async function createGraph() {
		try {
			displayedNodes = []
			edges = []

			nodes?.forEach((graphNode, index) => {
				displayedNodes.push({
					type: 'node',
					id: graphNode.id,
					position: {
						x: 80,
						y: 16 + index * 88
					},
					data: {
						custom: {
							component: DecisionTreeGraphNode,
							props: {
								node: graphNode,
								selected: selectedNodeId == graphNode.id
							},
							cb: (e: string, detail: any) => {
								console.log(e, detail)
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
					nodes={displayedNodes}
					width={fullSize ? fullWidth : width}
					{edges}
					{height}
					{scroll}
					nodeSelected={false}
					background={false}
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
