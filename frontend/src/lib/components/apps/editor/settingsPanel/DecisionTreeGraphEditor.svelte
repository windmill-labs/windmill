<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { Network } from 'lucide-svelte'
	import type { AppComponent, DecisionTreeGraph } from '../component'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Svelvet from '$lib/components/graph/svelvet/container/views/Svelvet.svelte'
	import type { UserEdgeType } from '$lib/components/graph/svelvet/types'
	import { NODE, type Node } from '$lib/components/graph'
	import DecisionTreeGraphNode from './DecisionTreeGraphNode.svelte'
	import { onMount } from 'svelte'

	export let component: AppComponent
	export let graph: DecisionTreeGraph

	let drawer: Drawer | undefined = undefined
	let nodes: Node[] = JSON.parse(
		'[{"type":"node","id":"Input","position":{"x":120.5,"y":42},"data":{"custom":{"props":{"label":"Input","insertable":true,"modules":[{"id":"a","value":{"type":"identity","flow":false}}],"bgColor":"#dfe6ee","selected":false,"index":0,"selectable":true,"center":true,"disableAi":false}}},"width":275,"height":34,"borderColor":"#999","sourcePosition":"bottom","targetPosition":"top","parentIds":[],"loopDepth":0,"childNodes":[]},{"type":"node","id":"a","position":{"x":120.5,"y":126},"data":{"custom":{"props":{"trigger":false,"mod":{"id":"a","value":{"type":"identity","flow":false}},"insertable":true,"insertableEnd":true,"branchable":false,"bgColor":"#fff","modules":[{"id":"a","value":{"type":"identity","flow":false}}],"disableAi":false}}},"width":275,"height":34,"parentIds":["Input"],"sourcePosition":"bottom","targetPosition":"top","loopDepth":0,"childNodes":[]},{"type":"node","id":"-2","position":{"x":120.5,"y":210},"data":{"custom":{"props":{"label":"Result","insertable":true,"bgColor":"#fff","selected":false,"index":1,"selectable":true,"center":true,"disableAi":false}}},"width":275,"height":34,"borderColor":"#999","sourcePosition":"bottom","targetPosition":"top","parentIds":["a"],"loopDepth":0,"childNodes":[]}]'
	)
	let edges: UserEdgeType[] = []
	let width: number, height: number
	let fullWidth: number
	let scroll = false
	let fullSize = false

	async function createGraph(graph: DecisionTreeGraph) {
		try {
			nodes = []
			edges = []

			graph.nodes.forEach((graphNode, index) => {
				nodes.push({
					type: 'node',
					id: graphNode.id,
					position: {
						x: 80,
						y: 24 + index * 64
					},
					data: {
						custom: {
							component: DecisionTreeGraphNode,
							props: {
								node: graphNode
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
		await createGraph({
			nodes: [
				{
					id: 'Input',
					label: 'Input',
					next: [
						{
							id: 'a',
							label: 'a'
						}
					]
				},
				{
					id: 'a',
					label: 'a',
					next: [
						{
							id: '-2',
							label: '-2'
						}
					]
				},
				{
					id: '-2',
					label: 'Result',
					next: []
				}
			]
		})
	})
</script>

<Drawer bind:this={drawer} on:close={() => {}} on:open={() => {}} size="800px">
	<DrawerContent
		title="Decision tree"
		on:close={drawer.closeDrawer}
		noPadding
		tooltip="Decision tree graph editor"
	>
		<Splitpanes>
			<Pane size={50} class="h-full">
				<Svelvet
					on:expand={() => {}}
					download={false}
					highlightEdges={false}
					locked
					dataflow={false}
					{nodes}
					width={fullSize ? fullWidth : width}
					{edges}
					{height}
					{scroll}
					nodeSelected={false}
					background={false}
				/>
			</Pane>
			<Pane size={50}>
				<p>test</p>
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
