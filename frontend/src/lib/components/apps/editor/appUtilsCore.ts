import type { AppInputs, EvalV2AppInput } from '../inputType'
import type { App, GridItem } from '../types'
import type { AppComponent } from './component'

export const BG_PREFIX = 'bg_'

export function findGridItemById(
	root: GridItem[],
	subGrids: Record<string, GridItem[]> | undefined,
	id: string
): GridItem | undefined {
	for (const gridItem of allItems(root, subGrids)) {
		if (gridItem.id === id) {
			return gridItem
		}
	}
	return undefined
}

export function allItems(
	grid: GridItem[],
	subgrids: Record<string, GridItem[]> | undefined
): GridItem[] {
	if (subgrids == undefined) {
		return grid ?? []
	}
	return [...(grid ?? []), ...Object.values(subgrids).flat()]
}

export function findGridItem(app: App, id: string): GridItem | undefined {
	return findGridItemById(app.grid, app.subgrids, id)
}
export function collectOneOfFields(fields: AppInputs, app: App): Record<string, any[]> {
	return Object.fromEntries(
		Object.entries(fields ?? {})
			.filter(([k, v]) => v.type == 'evalv2')
			.map(([k, v]) => {
				let field = v as EvalV2AppInput
				if (!field.connections || field.connections.length !== 1) {
					return [k, undefined]
				}
				const c = field.connections[0]

				const gridItem = findGridItem(app, c.componentId)

				if (field.expr !== c.componentId + '.' + c.id) {
					return [k, undefined]
				}

				if (gridItem) {
					const c = gridItem.data as AppComponent
					if (c) {
						if (
							c.type === 'resourceselectcomponent' ||
							c.type === 'selectcomponent' ||
							c.type === 'multiselectcomponent' ||
							c.type === 'multiselectcomponentv2'
						) {
							if (
								(c.type === 'selectcomponent' ||
									c.type === 'multiselectcomponent' ||
									c.type === 'multiselectcomponentv2') &&
								c.configuration?.create?.type === 'static' &&
								c.configuration?.create?.value === true
							) {
								return [k, undefined]
							}
							if (c.configuration?.items?.type === 'static') {
								const items = c.configuration.items.value
								if (items && Array.isArray(items)) {
									if (c.type === 'multiselectcomponent' || c.type === 'multiselectcomponentv2') {
										return [k, items]
									} else {
										const options = items
											.filter(
												(item) => item && typeof item === 'object' && 'value' in item && item.value
											)
											.map((item) => item.value)

										return [k, options]
									}
								}
							}
						}
					}
				}

				return [k, undefined]
			})
			.filter(([k, v]) => v !== undefined)
	)
}
