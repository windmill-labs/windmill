import type { SupportedLanguage } from "../../common"

export type ModuleHost = 'local' | 'hub'

export interface NodeSizeContext {
	w: number
	h: number
}

export type GraphItems = GraphNodeClass[][]

export interface NodeParent {
	row: number
	col: number
}

export function isParent(parent: any): parent is NodeParent {
	return typeof parent?.row === 'number' && typeof parent?.col === 'number'
}

export function isParentArray(parent: any): parent is NodeParent[] {
	return Array.isArray(parent) &&
		(parent?.length
			? typeof parent[0]?.row === 'number' && typeof parent[0]?.col === 'number'
			: true)
}

export class GraphNodeClass {
	isRoot: boolean
	topAnchor!: DOMPoint
	botAnchor!: DOMPoint

	constructor(public box: DOMRect, public parent: undefined | NodeParent | NodeParent[]) {
		this.isRoot = !!parent
		this.updateBox(box)
	}

	updateBox(box: DOMRect): void {
		const { top, bottom, left, width } = box
		const mid = left + width * 0.5
		this.topAnchor = new DOMPoint(mid, top)
		this.botAnchor = new DOMPoint(mid, bottom)
	}

	getParent(items: GraphItems): undefined | GraphNodeClass | GraphNodeClass[] {
		if(!this.parent) return undefined
		if(Array.isArray(this.parent)) {
			return this.parent.map(p => items[p.row][p.col])
		}
		return items[this.parent.row][this.parent.col]
	}

	getParentAnchor(items: GraphItems): undefined | DOMPoint | DOMPoint[] {
		const parent = this.getParent(items)
		if(!parent) return undefined
		if(Array.isArray(parent)) {
			return parent.map(p => p.botAnchor)
		}
		return parent.botAnchor
	}
}

export interface IFlowModule {
	node: GraphNodeClass
	title: string
	lang: SupportedLanguage
	host: ModuleHost
}