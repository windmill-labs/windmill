<script lang="ts">
	import Svelvet, { type Edge } from 'svelvet'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import { type FlowModule, FlowStatusModule, type ForloopFlow, RawScript } from '../../gen'
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
	} from '.'
	import { defaultIfEmptyString, truncateRev } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { numberToChars } from '../flows/utils'
	import type { FlowState } from '../flows/flowState'

	export let modules: FlowModule[] | undefined = []
	export let failureModule: FlowModule | undefined = undefined
	export let minHeight: number = 0
	export let notSelectable = false
	export let flowModuleStates: Record<string, FlowStatusModule.type> | undefined = undefined

	let selectedNode: string | undefined = undefined

	const idGenerator = createIdGenerator()
	let nestedNodes: NestedNodes
	let nodes: Node[] = []
	let edges: Edge[] = []
	let width: number, height: number

	let dispatch = createEventDispatcher()

	$: {
		width && height && minHeight && selectedNode && flowModuleStates
		if (modules) {
			createGraph(modules, failureModule)
		} else {
			nodes = edges = []
		}
	}

	function createGraph(modules: FlowModule[], failureModule?: FlowModule) {
		nestedNodes = nodes = []

		nestedNodes.push(createVirtualNode(getParentIds(), 'Flow start'))
		modules.forEach((m) => {
			const item = getConvertedFlowModule(m)
			item && nestedNodes.push(item)
		})
		const endParentIds = getParentIds()
		nestedNodes.push(createVirtualNode(getParentIds(), 'Flow end'))
		if (failureModule) {
			nestedNodes.push(createErrorHandler(endParentIds, failureModule))
		}

		const flatNodes = flattenNestedNodes(nestedNodes)
		const layered = layoutNodes(flatNodes)
		nodes = layered.nodes
		// width = layered.width
		height = layered.height
		edges = createEdges(nodes)
	}

	function getConvertedFlowModule(
		module: FlowModule,
		parent: NestedNodes | string | undefined = undefined,
		edgeLabel: string | undefined = undefined
	): GraphItem | undefined {
		const type = module.value.type
		const parentIds = getParentIds(parent)

		if (type === 'rawscript') {
			const lang = module.value.language
			return flowModuleToNode(
				parentIds,
				module.id,
				module.summary || 'Inline script',
				'inline',
				module,
				lang,
				edgeLabel
			)
		} else if (type === 'script') {
			const isHub = module.value.path.startsWith('hub/')
			return flowModuleToNode(
				parentIds,
				module.id,
				module.summary || module.value.path,
				isHub ? 'hub' : 'workspace',
				module,
				undefined,
				edgeLabel
			)
		} else if (type === 'forloopflow') {
			return flowModuleToLoop(module.value.modules, module, parent)
		} else if (type === 'branchone') {
			const branches = [module.value.default, ...module.value.branches.map((b) => b.modules)]
			return flowModuleToBranch(
				module,
				branches,
				['Default', ...module.value.branches.map((x) => `If ${truncateRev(x.expr, 20)}`)],
				parent
			)
		} else if (type === 'branchall') {
			const branches = module.value.branches.map((b) => b.modules)
			return flowModuleToBranch(module, branches, [], parent)
		}
		return flowModuleToNode(
			parentIds,
			module.id,
			module.summary || 'Identity step',
			'inline',
			module,
			undefined,
			edgeLabel
		)
	}

	function getParentIds(items: string | NestedNodes | undefined = undefined): string[] {
		if (typeof items == 'string') {
			return [items]
		}
		const item = items?.at(-1) || nestedNodes.at(-1)
		if (!item) return []

		if (isNode(item)) {
			return ['' + item.id]
		} else if (isLoop(item)) {
			return getParentIds(item.items)
		} else if (isBranch(item)) {
			return item.items.map((i) => getParentIds(i)).flat()
		}
		return []
	}

	function getStateColor(state: FlowStatusModule.type | undefined): string {
		switch (state) {
			case FlowStatusModule.type.SUCCESS:
				return 'rgb(34 197 94)'
			case FlowStatusModule.type.FAILURE:
				return 'rgb(248 113 113)'
			case FlowStatusModule.type.IN_PROGRESS:
				return 'rgb(253 224 71)'
			default:
				return '#fff'
		}
	}
	function flowModuleToNode(
		parentIds: string[],
		id: string,
		title: string,
		host: ModuleHost,
		onClickDetail: any,
		lang?: RawScript.language,
		edgeLabel?: string
	): Node {
		const langImg: Record<RawScript.language, string> = {
			deno: '/icons/ts-lang.svg',
			go: '/icons/go-lang.svg',
			python3: '/icons/python-lang.svg',
			bash: '/icons/bash-lang.svg'
		}
		const hostImg: Record<ModuleHost, string> = {
			hub: '/icons/hub-script.svg',
			workspace: '/icons/inline-script.svg',
			inline: ''
		}
		const wrapperWidth = lang ? 'w-[calc(100%-70px)]' : 'w-[calc(100%-50px)]'
		const graphId = idGenerator.next().value
		return {
			id: graphId,
			position: { x: -1, y: -1 },
			data: {
				html: `
				<div class="w-full flex justify-between items-center px-1">
					<div class="${wrapperWidth} text-left ellipsize text-2xs truncate">
						${title}
					</div>
					<div class="flex items-center">
						${lang ? `<img src="${langImg[lang]}" class="grayscale">` : ''}
						${host != 'inline' ? `<img src="${hostImg[host]}" class="grayscale">` : ''}
						<span class="center-center font-semibold bg-indigo-100 text-indigo-800 rounded px-1 pb-[2px] ml-[2px]">
							${id ?? numberToChars(graphId - 1)}
						</span>
					</div>
				</div>
			`
			},
			host,
			width: NODE.width,
			height: NODE.height,
			borderColor: selectedNode == onClickDetail.id ? 'black' : '#999',
			bgColor:
				selectedNode == onClickDetail.id
					? '#f5f5f5'
					: getStateColor(flowModuleStates?.[onClickDetail.id]),
			parentIds,
			clickCallback: (node) => {
				if (!notSelectable) {
					selectedNode = onClickDetail.id
				}
				dispatch('click', onClickDetail)
			},
			edgeLabel
		}
	}

	function flowModuleToLoop(
		modules: FlowModule[],
		module: FlowModule,
		parent: NestedNodes | string | undefined = undefined
	): Loop {
		const value = module.value as ForloopFlow
		const expr = value.iterator.type == 'static' ? value.iterator.value : value.iterator.expr
		const loop: Loop = {
			type: 'loop',
			items: [
				flowModuleToNode(
					getParentIds(parent),
					module.id,
					module.summary || `For Loop: ${defaultIfEmptyString(expr ?? '', 'TBD')}`,
					'inline',
					module,
					undefined
				)
			]
		}
		modules.forEach((module) => {
			const item = getConvertedFlowModule(module, loop.items)
			item && loop.items.push(item)
		})
		loop.items.push(
			createVirtualNode(
				getParentIds(loop.items),
				`Collect iterations' results of For Loop ${module.id}`
			)
		)
		return loop
	}

	function flowModuleToBranch(
		module: FlowModule,
		branches: FlowModule[][],
		edgesLabel: string[],
		parent: string | NestedNodes | undefined = undefined
	): Branch {
		const branch: Branch = {
			type: 'branch',
			node: flowModuleToNode(
				getParentIds(parent),
				module.id,
				module.summary || module.value.type == 'branchall'
					? 'Run all branches'
					: 'Run one branch given predicate',
				'inline',
				module,
				undefined
			),
			items: []
		}
		const branchParent = [branch.node.id.toString()]
		if (branches.length == 0) {
			branch.items.push([createVirtualNode(branchParent, 'No branches')])
		}
		branches.forEach((modules, i) => {
			const items: NestedNodes = []
			if (!modules.length) {
				items.push(createVirtualNode(branchParent, 'Empty branch', edgesLabel[i]))
			} else {
				modules.forEach((module) => {
					const item = getConvertedFlowModule(
						module,
						items.length ? items : branch.node.id.toString(),
						edgesLabel[i]
					)
					item && items.push(item)
				})
			}
			items.length && branch.items.push(items)
		})
		return branch
	}

	function flattenNestedNodes(nestedNodes: NestedNodes, nodes: Node[] = []): Node[] {
		const array = nodes
		nestedNodes.forEach((node) => {
			if (isNode(node)) {
				array.push(node)
			} else if (isLoop(node)) {
				flattenNestedNodes(node.items, array)
			} else if (isBranch(node)) {
				array.push(node.node)
				node.items.forEach((item) => {
					flattenNestedNodes(item, array)
				})
			}
		})
		return array
	}

	function layoutNodes(nodes: Node[]): { nodes: Node[]; height: number } {
		const stratify = dagStratify().id(({ id }: Node) => '' + id)
		const dag = stratify(nodes)
		const layout = sugiyama()
			.decross(decrossOpt())
			.coord(coordCenter())
			.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
		const boxSize = layout(dag)
		return {
			nodes: dag.descendants().map((des) => ({
				...des.data,
				id: +des.data.id,
				position: {
					x: des.x ? des.x + (width - boxSize.width - NODE.width) / 2 : 0,
					y: des.y || 0
				}
			})),
			height: Math.max(boxSize.height + NODE.height, minHeight)
		}
	}

	function createEdges(nodes: Node[]): Edge[] {
		const edges: Edge[] = []
		nodes.forEach((node) => {
			node.parentIds.forEach((pid, i) => {
				edges.push({
					id: `e-${pid}-${node.id}`,
					source: +pid,
					target: node.id,
					labelBgColor: 'white',
					arrow: true,
					animate: false,
					noHandle: false,
					label: node.edgeLabel
					// type: 'smoothstep'
				})
			})
		})
		return edges
	}

	function createVirtualNode(parentIds: string[], label: string, edgesLabel?: string): Node {
		return {
			id: idGenerator.next().value,
			position: { x: -1, y: -1 },
			data: {
				html: `
				<div class="w-full max-h-full text-center ellipsize-multi-line text-2xs [-webkit-line-clamp:2] px-1">
					${label}
				</div>
			`
			},
			width: NODE.width,
			height: NODE.height,
			borderColor: '#999',
			bgColor: '#d4e4ff',
			parentIds
		}
	}

	function createErrorHandler(parentIds: string[], module: FlowModule): Node {
		return {
			id: -1,
			position: { x: -1, y: -1 },
			data: {
				html: `
				<div class="w-full max-h-full text-center ellipsize-multi-line text-2xs [-webkit-line-clamp:2] px-1">
					Error handler
				</div>
			`
			},
			width: NODE.width,
			height: NODE.height,
			bgColor: 'rgb(248 113 113)',
			borderColor: '#999',

			parentIds,
			clickCallback: (node) => {
				if (!notSelectable) {
					selectedNode = module.id
				}
				dispatch('click', module)
			}
		}
	}
</script>

<div bind:clientWidth={width} class="w-full h-full overflow-hidden">
	{#if width && height}
		<Svelvet {nodes} {width} {edges} {height} bgColor="rgb(249 250 251)" />
	{/if}
</div>
