<script lang="ts">
	import Svelvet, { type Edge } from "svelvet";
	import { sugiyama, dagStratify, decrossOpt } from 'd3-dag'
	import type { FlowModule, RawScript } from "../../gen"
	import {
		NODE,
		createIdGenerator,
		isNode,
		isLoop,
		isBranch,
		type GraphItem,
		type Node,
		type Loop,
		type Branch,
		type NestedNodes,
		type ModuleHost
	} from "."

	export let modules: FlowModule[] | undefined = []
	const idGenerator = createIdGenerator()
	let nestedNodes: NestedNodes
	let nodes: Node[] = []
	let edges: Edge[] = []
	let width: number, height: number
	
	$: if(modules?.length) {
		createGraph(modules)
	} else {
		nodes = edges = []
	}

	function createGraph(modules: FlowModule[]) {
		nestedNodes = nodes = []

		nestedNodes.push(createVirtualNode(getParentIds(), 'Flow start'))
		modules.forEach(m => {
			const item = getConvertedFlowModule(m)
			item && nestedNodes.push(item)
		})
		nestedNodes.push(createVirtualNode(getParentIds(), 'Flow end'))

		const flatNodes = flattenNestedNodes(nestedNodes)
		nodes = layoutNodes(flatNodes)
		edges = createEdges(nodes)
	}

	function getConvertedFlowModule(module: FlowModule, parent: NestedNodes | undefined = undefined): GraphItem | undefined {
		const type = module.value.type
		const parentIds = getParentIds(parent)

		if(type === 'rawscript') {
			const lang = module.value.language
			return flowModuleToNode(parentIds, module.summary || 'Inline script', 'inline', lang)
		} else if(type === 'script') {
			return flowModuleToNode(parentIds, module.summary || module.value.path || 'Hub script', 'hub')
		} else if(type === 'forloopflow') {
			const expr = module.value.iterator['expr']
			return flowModuleToLoop(module.value.modules, expr ? `Expression: ${expr}` : 'For loop', parent)
		} else if(type === 'branchone') {
			const branches = [module.value.default, ...module.value.branches.map(b => b.modules)]
			return flowModuleToBranch(branches, parent)
		} else if(type === 'branchall') {
			const branches = module.value.branches.map(b => b.modules)
			return flowModuleToBranch(branches, parent)
		}
		return undefined
	}

	function getParentIds(items: NestedNodes | undefined = undefined): string[] {
		const item = items?.at(-1) || nestedNodes.at(-1)
		if(!item) return []

		if(isNode(item)) {
			return ['' + item.id]
		} else if(isLoop(item)) {
			return getParentIds(item.items)
		} else if(isBranch(item)) {
			return item.items.map(i => getParentIds(i)).flat()
		}
		return []
	}

	function flowModuleToNode(
		parentIds: string[],
		title: string,
		host: ModuleHost,
		lang?: RawScript.language
	): Node {
		const langImg: Record<RawScript.language, string> = {
			deno: '/icons/ts-lang.svg',
			go: '/icons/go-lang.svg',
			python3: '/icons/python-lang.svg',
		}
		const hostImg: Record<ModuleHost, string> = {
			hub: '/icons/hub-script.svg',
			inline: '/icons/inline-script.svg',
		}
		const wrapperWidth = lang ? 'w-[calc(100%-40px)]' : 'w-[calc(100%-20px)]'
		return {
			id: idGenerator.next().value,
	    position: { x: -1, y: -1 },
	    data: { html: `
				<div class="w-full flex justify-between items-center px-1">
					<div class="${wrapperWidth} text-left ellipsize">${title}</div>
					<div class="flex items-center grayscale">
						${lang ? `<img src="${langImg[lang]}">` : ''}
						<img src="${hostImg[host]}">
					</div>
				</div>
			`},
			host,
	    width: NODE.width,
	    height: NODE.height,
	    bgColor: "white",
			parentIds
		}
	}

	function flowModuleToLoop(modules: FlowModule[], startLabel: string, parent: NestedNodes | undefined = undefined): Loop {
		const loop: Loop = {
			type: 'loop',
			items: [createVirtualNode(getParentIds(parent), startLabel)]
		}
		modules.forEach(module => {
			const item = getConvertedFlowModule(module, loop.items)
			item && loop.items.push(item)
		})
		loop.items.push(createVirtualNode(getParentIds(loop.items), 'Collection of the results'))
		return loop
	}

	function flowModuleToBranch(branches: FlowModule[][], parent: NestedNodes | undefined = undefined): Branch {
		const branch: Branch = {
			type: 'branch',
			items: []
		}
		branches.forEach(modules => {
			const items: NestedNodes = []
			modules.forEach(module => {
				const item = getConvertedFlowModule(module, items.length ? items : parent)
				item && items.push(item)
			})
			items.length && branch.items.push(items)
		})
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
		const stratify = dagStratify().id(({id}: Node) => '' + id)
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
			node.parentIds.forEach(pid => {
				edges.push({
					id: `e-${pid}-${node.id}`,
					source: +pid,
					target: node.id,
					arrow: true,
					// type: 'smoothstep'
				})
			})
		})
		return edges
	}

	function createVirtualNode(parentIds: string[], label: string): Node {
		return {
			id: idGenerator.next().value,
	    position: { x: -1, y: -1 },
	    data: { label },
	    width: NODE.width,
	    height: NODE.height,
	    bgColor: "#d4e4ff",
			parentIds
		}
	}
</script>

<div bind:clientWidth={width} bind:clientHeight={height} class="w-full h-full">
	{#if width && height}
		<Svelvet {nodes} {edges} width={width-1} height={height-1} background />
	{/if}
</div>