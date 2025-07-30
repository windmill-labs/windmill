/**
 * Base class for embedding a svelte component within an AGGrid call.
 * See: https://stackoverflow.com/a/72608215
 */
import type { ColDef, ColGroupDef, ICellRendererComp, ICellRendererParams } from 'ag-grid-community'
import { ColumnIdentity, type ColumnDef } from '../dbtable/utils'
import type { TableAction } from '$lib/components/apps/editor/component'
import { mount, unmount } from 'svelte'
import { Button } from '$lib/components/common'
import { Trash2 } from 'lucide-svelte'

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

export function transformColumnDefs({
	columnDefs,
	actions,
	customActionsHeader,
	wrapActions,
	tableActionsFactory,
	onDelete,
	onInvalidColumnDefs
}: {
	columnDefs: ColumnDef[]
	actions?: TableAction[]
	customActionsHeader?: string
	wrapActions?: boolean
	tableActionsFactory?: ReturnType<typeof cellRendererFactory>
	onDelete?: (values: object) => void
	onInvalidColumnDefs?: (errors: string[]) => void
}): (ColDef<unknown, any> | ColGroupDef<unknown>)[] {
	if (!columnDefs) {
		return []
	}

	const { isValid, errors } = validateColumnDefs(columnDefs)

	if (!isValid) {
		onInvalidColumnDefs?.(errors)
		return []
	}

	let r: any[] = columnDefs?.filter((x) => x && !x.ignored) ?? []

	if (onDelete) {
		r.push({
			field: 'delete',
			headerName: '',
			cellRenderer: cellRendererFactory((c, p) => {
				const btnComponent = mount(Button, {
					target: c.eGui,
					props: {
						btnClasses: 'w-12 bg-transparent rounded-none h-full hover:bg-red-500',
						wrapperClasses: 'flex justify-center items-center absolute inset-0 h-full',
						color: 'light',
						size: 'sm',
						variant: 'contained',
						iconOnly: true,
						startIcon: { icon: Trash2 },
						nonCaptureEvent: true
					}
				})
				return {
					destroy: () => {
						unmount(btnComponent)
					},
					refresh(params) {
						//
					}
				}
			}),
			cellRendererParams: {
				onClick: (e) => {
					onDelete?.(e)
				}
			},
			lockPosition: 'right',
			editable: false,
			flex: 0,
			width: 50,
			minWidth: 50,
			resizable: false,
			pinned: 'right'
		})
	}

	if (actions?.length) {
		r.push({
			headerName: customActionsHeader ?? 'Actions',
			cellRenderer: tableActionsFactory,
			autoHeight: true,
			cellStyle: { textAlign: 'center' },
			cellClass: 'grid-cell-centered',
			lockPosition: 'right',

			...(!wrapActions ? { minWidth: 130 * actions?.length } : {})
		})
	}

	return r.map((fields) => {
		let cr = defaultCellRenderer(fields.cellRendererType)
		return {
			...fields,
			...(cr ? { cellRenderer: cr } : {})
		}
	})
}

export function validateColumnDefs(columnDefs: ColumnDef[]): {
	isValid: boolean
	errors: string[]
} {
	let isValid = true
	const errors: string[] = []

	if (!Array.isArray(columnDefs)) {
		return { isValid: false, errors: ['Column definitions must be an array.'] }
	}

	// Validate each column definition
	columnDefs.forEach((colDef, index) => {
		let noField = !colDef.field || typeof colDef.field !== 'string' || colDef.field.trim() === ''

		if (
			(colDef.isidentity === ColumnIdentity.ByDefault ||
				colDef.isidentity === ColumnIdentity.Always) &&
			colDef.hideInsert == undefined
		) {
			colDef.hideInsert = true
		}

		// Check if 'field' property exists and is a non-empty string
		if (noField && !(colDef.children && Array.isArray(colDef.children))) {
			isValid = false
			errors.push(
				`Column at index ${index} is missing a valid 'field' property nor having any children.`
			)
		}

		if (colDef.children && Array.isArray(colDef.children)) {
			const { isValid: isChildrenValid, errors: childrenErrors } = validateColumnDefs(
				colDef.children
			)
			if (!isChildrenValid) {
				isValid = false
				errors.push(...childrenErrors.map((err) => `Error in children at index ${index}: ${err}`))
			}
		}
	})

	return { isValid, errors }
}
