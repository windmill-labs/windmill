import { select } from "d3-selection"
import { writable } from "svelte/store"
import type { FlowItem, FlowItemsGraph } from "./model"

export function ellipsize(text: string, element: Element, maxWidth: number, cutoff = '...') {
	const elem = select(element)
	elem.attr('title', text)
	let currentWidth = (<SVGTextElement>elem.text(text).node())?.getBoundingClientRect().width
	if (!currentWidth || currentWidth < maxWidth) {
		return
	}

	const cl = cutoff.length
	text = cutoff + text
	while ((currentWidth > maxWidth) && text.length) {
		text = text.slice(0, -1).trimEnd()
		elem.text(text.slice(cl) + cutoff)
		currentWidth = (<SVGTextElement>elem.node()).getBoundingClientRect().width
	}
}

export function createGraphStore(initial: FlowItemsGraph = []) {
	const store = writable<FlowItemsGraph>(initial)
	return {
		subscribe: store.subscribe,
		set: store.set,
		reset: () => store.set(initial),
		update: store.update,
		append: (module: FlowItem | undefined) => {
			if(!module) return
			store.update(last => [...last, [module]])
		}
	}
}