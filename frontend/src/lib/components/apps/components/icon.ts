import { toKebabCase } from '../utils'

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
