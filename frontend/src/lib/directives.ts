export function skeleton(
	node: HTMLElement,
	loading: boolean,
	{ width, height }: { width?: number | string; height?: number | string } = {}
): { destroy(): void } {
	const classes = ['animate-pulse', 'bg-blue-200', 'rounded']

	if (loading) {
		if (typeof node?.querySelectorAll === 'function') {
			const targets = Array.from(node.querySelectorAll('*:not(:has(*))'))
			targets.forEach((t) => t.classList.add(...classes))
		}
		if (width) {
			node.style.width = width + (typeof width === 'number' ? 'px' : '')
		}
		if (height) {
			node.style.height = height + (typeof height === 'number' ? 'px' : '')
		}
	} else {
		if (typeof node?.querySelectorAll === 'function') {
			const query = '.' + classes.join(' .')
			const targets = Array.from(node.querySelectorAll(query))
			targets.forEach((t) => t.classList.remove(...classes))
		}
		node.style.removeProperty('width')
		node.style.removeProperty('height')
	}

	return {
		destroy() {}
	}
}
