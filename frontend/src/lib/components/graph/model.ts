import type { Writable } from "svelte/store"
import type { Node } from "svelvet"
import type { SupportedLanguage } from "../../common"

export type ModuleHost = 'inline' | 'hub'

export const GRAPH_CONTEXT_KEY = 'graph_context' as const
export interface GraphContext {
	graph: Writable<FlowItemsGraph>
	node: {
		width: number,
		height: number,
	},
	loop: {
		width: number,
		padding: number,
		scale: number,
	},
	gap: {
		vertical: number,
		horizontal: number,
	}
}

export type GraphItems = GraphNodeClass[][]

export interface NodeIndex {
	row: number
	col: number
}

export function isParent(parent: GraphNodeParent): parent is GraphNodeClass {
	return (parent instanceof GraphNodeClass)
}

export function isParentArray(parent: GraphNodeParent): parent is GraphNodeClass[] {
	return Array.isArray(parent) &&
		(parent?.length ? parent[0] instanceof GraphNodeClass : true)
}

export type GraphNodeParent = undefined | GraphNodeClass | GraphNodeClass[]

export class GraphNodeClass {
	isRoot: boolean
	box!: DOMRect
	topAnchor!: DOMPoint
	botAnchor!: DOMPoint
	private _parent: GraphNodeParent
	get parent() { return this._parent }

	constructor(box: { x?: number, y?: number, width?: number, height?: number }, parent: GraphNodeParent) {
		this.isRoot = !parent
		this._parent = parent
		this.updateBox(box)
	}

	updateBox(update: { x?: number, y?: number, width?: number, height?: number }): void {
		const x = update.x || this.box?.x
		const y = update.y || this.box?.y
		const width = update.width || this.box?.width
		const height = update.height || this.box?.height
		this.box = new DOMRect(x, y, width, height)
		const mid = this.box.left + this.box.width * 0.5
		this.topAnchor = new DOMPoint(mid, this.box.top)
		this.botAnchor = new DOMPoint(mid, this.box.bottom)
	}

	setParent(parent: GraphNodeParent, updatePosition: boolean, gap: number = 0) {
		this.isRoot = !parent
		this._parent = parent
		if(updatePosition) {
			const botParent = this.getBottomMostParent()
			botParent && this.updateBox({y: botParent.box.bottom + gap})
		}
	}

	getParentAnchor(side: 'top' | 'bot' = 'bot'): undefined | DOMPoint | DOMPoint[] {
		if(!this.parent) return undefined
		const anchor = side + 'Anchor'
		if(Array.isArray(this.parent)) {
			return this.parent.map(p => p[anchor])
		}
		return this.parent[anchor]
	}

	getBottomMostParent(): undefined | GraphNodeClass {
		if(!this.parent) return undefined

		let bottomMostParent: GraphNodeClass
		if(Array.isArray(this.parent)) {
			bottomMostParent = this.parent[0]
			if(!bottomMostParent) return
			for (const p of this.parent) {
				if(p.box.bottom < bottomMostParent.box.bottom) bottomMostParent = p
			}
		} else {
			bottomMostParent = this.parent
		}
		return bottomMostParent
	}
}

export interface IFlowModuleNode {
	node: GraphNodeClass
	title: string
	host: ModuleHost
	lang?: SupportedLanguage
}

export function isFlowModuleNode(item: FlowItem | undefined): item is IFlowModuleNode {
	if(!item) return false
	return !!item['title'] && !!item['host'] && item['node'] instanceof GraphNodeClass
}

export interface IFlowLoopNode {
	node: GraphNodeClass
	title: string
	modules: FlowItem[]
}

export function isFlowLoopNode(item: FlowItem | undefined): item is IFlowLoopNode {
	if(!item) return false
	return item['node'] && item['title'] && item['modules'] && Array.isArray(item['modules'])
}

export interface IFlowBranchNode {
	node: GraphNodeClass
	modules: FlowItem[]
}

export function isFlowBranchNode(item: FlowItem | undefined): item is IFlowBranchNode {
	if(!item) return false
	return !item['title'] && item['modules'] && Array.isArray(item['modules'])
}

export type FlowItem = (IFlowModuleNode | IFlowLoopNode | IFlowBranchNode)

export type FlowItemsGraph = FlowItem[][]

//////////////////////////////////////////////////////////////////////

export type InnerItem = Node | Loop | Branch

export type Loop = {
	type: 'loop',
	items: NestedNodes
}

export type Branch = {
	type: 'branch',
	items: NestedNodes[]
}

export type GraphItem = Node | Loop | Branch

export type NestedNodes = GraphItem[]

export function isNode(node: GraphItem | NestedNodes | undefined): node is Node {
	return !Array.isArray(node) && typeof node?.['id'] === 'number'
}