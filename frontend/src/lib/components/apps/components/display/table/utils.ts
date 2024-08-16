/**
 * Base class for embedding a svelte component within an AGGrid call.
 * See: https://stackoverflow.com/a/72608215
 */
import type { ICellRendererComp, ICellRendererParams } from 'ag-grid-community'

/**
 * Class for defining a cell renderer.
 * If you don't need to define a separate class you could use cellRendererFactory
 * to create a component with the column definitions.
 */
export abstract class AbstractCellRenderer implements ICellRendererComp {
	eGui: any
	protected component:
		| {
				refresh: (params: ICellRendererParams) => void
				destroy: () => void
		  }
		| undefined
	constructor(parentElement = 'span') {
		// create empty span (or other element) to place svelte component in
		this.component = undefined
		this.eGui = document.createElement(parentElement)
	}

	init(params: ICellRendererParams & { onClick?: (data: any) => void }) {
		this.component = this.createComponent(params)
		this.eGui.addEventListener('click', () => params.onClick?.(params.data))
	}

	getGui() {
		return this.eGui
	}

	refresh(params: ICellRendererParams) {
		this.component?.refresh?.(params)
		return true
	}

	destroy(): void {
		this.component?.destroy?.()
	}
	/**
	 * Define and create the svelte component to use in the cell
	 * @example
	 * // This is all you need to do within this method: create the component with new, specify the target
	 * // is the class, and pass in props via the params.
	 * new CampusIcon({
	 *    target: this.eGui,
	 *    props: {
	 *        color: params.data?.color,
	 *        name: params.data?.name
	 * }
	 * @param params params for rendering the call, including the value for the cell
	 */
	abstract createComponent(params: ICellRendererParams): {
		refresh: (params: ICellRendererParams) => void
		destroy: () => void
	}
}

/**
 * Creates a cell renderer using the given callback for how to initialise a svelte component.
 * See AbstractCellRenderer.createComponent
 * @param svelteComponent function for how to create the svelte component
 * @returns
 */
export function cellRendererFactory(
	svelteComponent: (
		cell: AbstractCellRenderer,
		params: ICellRendererParams
	) => {
		refresh: (params: ICellRendererParams) => void
		destroy: () => void
	}
) {
	class Renderer extends AbstractCellRenderer {
		createComponent(params: ICellRendererParams<any, any>): {
			refresh: (params: ICellRendererParams) => void
			destroy: () => void
		} {
			return svelteComponent(this, params)
		}
	}
	return Renderer
}

export type LinkObject = {
	href: string
	label: string
}

export function isLinkObject(value: any): value is LinkObject {
	return value && typeof value === 'object' && 'href' in value && 'label' in value
}

export function defaultCellRenderer(cellRendererType: string) {
	if (cellRendererType === 'link') {
		return (params: ICellRendererParams) => {
			if (isLinkObject(params.value)) {
				const value = params.value
				return `<a href=${value.href} class="underline" target="_blank">${value.label}</a>`
			} else if (params.value) {
				return `<a href=${params.value} class="underline" target="_blank">${params.value}</a>`
			} else {
				return params.value
			}
		}
	} else {
		return undefined
	}
}
