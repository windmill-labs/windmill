function toKebabCase(str: string): string {
	return str
		.replace(/([a-z0-9])([A-Z])/g, '$1-$2') // Convert camel case to kebab case
		.replace(/([A-Z0-9])([A-Z])/g, '$1-$2') // Separate consecutive uppercase letters
		.replace(/([a-z])([0-9])/g, '$1-$2') // Separate letters from digits
		.toLowerCase()
}

export async function loadIcon(
	name: string,
	parent: HTMLElement,
	size: number | undefined,
	strokeWidth: number | undefined,
	color: string | undefined
): Promise<any> {
	try {
		if (name) {
			await fetch(
				`https://cdn.jsdelivr.net/npm/lucide-static@0.367.0/icons/${toKebabCase(name)}.svg`
			)
				.then((response) => response.text())
				.then((svg) => (parent.innerHTML = svg))
			let elem = parent.children?.[0]
			if (elem) {
				elem.setAttribute('width', size?.toString() ?? '24')
				elem.setAttribute('height', size?.toString() ?? '24')
				if (strokeWidth) {
					elem.setAttribute('stroke-width', strokeWidth.toString())
				}
				if (color) {
					elem.setAttribute('stroke', color)
				}
			}
		}
	} catch (error) {
		console.error(error)
	}
}
