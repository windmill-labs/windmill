import { select } from "d3-selection"

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