function toKebabCase(str: string): string {
	return str
		.replace(/([a-z0-9])([A-Z])/g, '$1-$2') // Convert camel case to kebab case
		.replace(/([A-Z0-9])([A-Z])/g, '$1-$2') // Separate consecutive uppercase letters
		.replace(/([a-z])([0-9])/g, '$1-$2') // Separate letters from digits
		.toLowerCase()
}

export async function loadIcon(name: string): Promise<any> {
	let iconComponent: any
	try {
		if (name) {
			iconComponent = (
				await import(
					`../../../../../node_modules/lucide-svelte/dist/svelte/icons/${toKebabCase(name)}.svelte`
				)
			).default
			return iconComponent
		} else {
			iconComponent = undefined
		}
	} catch (error) {
		console.error(error)
		iconComponent = undefined
	}
}
