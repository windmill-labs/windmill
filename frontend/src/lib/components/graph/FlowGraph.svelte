<script lang="ts">
	import Svelvet, { type Edge } from "svelvet";
	import { sugiyama, dagStratify, decrossOpt } from 'd3-dag'
	import type { FlowModule, RawScript } from "../../gen"
	import type { NestedNodes, ModuleHost } from "."
	import { isNode, isLoop, isBranch, type GraphItem, type Node, type Loop, type Branch } from "./model"

	const NODE = {
		width: 200,
		height: 40,
		gap: {
			horizontal: 20,
			vertical: 50
		}
	}

	export let modules: FlowModule[] = []
	let lastId: number | undefined = undefined
	const idGenerator = function*() {
		let id = 0
		while(true) {
			lastId = id === 0 ? undefined : id - 1
			yield id++
	}}()
	let nodes: Node[] = []
	let edges: Edge[] = []
	let width: number, height: number
	
	$: if(modules) createGraph(modules)

	function createGraph(modules: FlowModule[]) {
		let localLastId: number | undefined = undefined
		const nestedNodes: NestedNodes = nodes = []

		modules.forEach(m => {
			const item = getConvertedFlowModule(localLastId, m)
			if(!item) return;

			const isItemNode = isNode(item)
			if(isItemNode) {
				localLastId = item.id
			} else if(isLoop(item)) {
				localLastId = lastId
			}
			nestedNodes.push(item)
			if(!isItemNode) {
				if(isBranch(item)) {
					localLastId = lastId
				}
			}
		})

		const flatNodes = flattenNestedNodes(nestedNodes)
		nodes = layoutNodes(flatNodes)
		edges = createEdges(nodes)
	}

	function getConvertedFlowModule(parentId: number | undefined, module: FlowModule): GraphItem {
		const type = module.value.type

		if(type === 'rawscript') {
			const lang = module.value.language
			return flowModuleToNode(parentId, module.summary || 'Inline ' + lang, 'inline', lang)
		} else if(type === 'script') {
			return flowModuleToNode(parentId, module.summary || module.value.path, 'hub')
		} /*else if(type === 'forloopflow') {
			return flowModuleToLoop(parentLevel, module.value.modules, depth, module.summary)
		}*/ else if(type === 'branchone') {
			const branches = [module.value.default, ...module.value.branches.map(b => b.modules)]
			return flowModuleToBranch(parentId, branches)
		} /*else if(type === 'branchall') {}*/
		return {type: 'loop', items: []}
	}

	function flowModuleToNode(
		parentId: number | undefined,
		title: string,
		host: ModuleHost,
		lang?: RawScript.language
	): Node {
		return {
			id: idGenerator.next().value,
	    position: { x: -1, y: -1 },
	    data: { label: `${title} - ${host}` },
			host,
	    width: NODE.width,
	    height: NODE.height,
	    bgColor: "white",
			childNodes: [],
			parentIds: parentId ? ['' + parentId] : []
		}
	}

	function flowModuleToBranch(parentId: number | undefined, branches: FlowModule[][]): Branch {
		const branch: Branch = {
			type: 'branch',
			items: []
		}
		for (let i = 0; i < branches.length; i++) {
			const items: GraphItem[] = []
			for (let j = 0; j < branches[i].length; j++) {
				const item = getConvertedFlowModule(items.at(-1)?.['id'] || parentId, branches[i][j])
				item && items.push(item)
			}
			if(items.length) {
				branch.items.push(items)
			}
		}
		return branch
	}

	function flattenNestedNodes(nestedNodes: NestedNodes, nodes: Node[] = []): Node[] {
		const array = nodes
		nestedNodes.forEach(node => {
			if(isNode(node)) {
				array.push(node)
			} else if (isLoop(node)) {
				flattenNestedNodes(node.items, array)
			} else if(isBranch(node)) {
				node.items.forEach(item => {
					flattenNestedNodes(item, array)
				})
			}
		})
		
		return array
	}

	function layoutNodes(nodes: Node[]) {
		if(!nodes.length) return []
		const stratify = dagStratify().id(({id}: Node) => '' + id).parentIds(({parentNode}: Node) => parentNode ? ['' + parentNode] : [])
		const dag = stratify(nodes)
		const layout = sugiyama()
			.decross(decrossOpt())
			.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
		layout(dag)
		
		return dag
			.descendants()
			.map(des => ({...des.data, id: +des.data.id, position: { x: des.x || 0, y: des.y || 0 }}))
	}

	function createEdges(nodes: Node[]): Edge[] {
		const edges: Edge[] = []
		nodes.forEach(node => {
			if(typeof node.parentNode !== 'number') return
			edges.push({
				id: `e-${node.parentNode}-${node.id}`,
				source: node.parentNode,
				target: node.id,
				arrow: true
			})
		})

		return edges
	}
</script>

<div bind:clientWidth={width} bind:clientHeight={height} class="w-full h-full">
	{#if width && height}
		<Svelvet {nodes} {edges} {width} {height} background />
	{/if}
</div>
