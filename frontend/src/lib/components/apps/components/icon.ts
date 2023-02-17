import { toKebabCase } from '../utils'

export async function loadIcon(name: string): Promise<any> {
    let iconComponent: any
    try {
        if (name) {
            name = toKebabCase(name).replace(/([a-z])(\d)/i, '$1-$2')
            iconComponent = (
                await import(
                    `../../../../../node_modules/lucide-svelte/dist/svelte/icons/${name}.svelte`
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