import type { Schema } from '$lib/common'
import { FlowService, ScriptService } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { emptySchema } from '$lib/utils'
import type { AppComponent } from './editor/component'
import type { App, ComponentCssProperty, ComponentCustomCSS, GridItem } from './types'
import { getRecommendedDimensionsByComponent } from './editor/component'
import { gridColumns } from './gridUtils'
import gridHelp from '@windmill-labs/svelte-grid/src/utils/helper'
import { twMerge } from 'tailwind-merge'
import type { AppInput, InputType, ResultAppInput, StaticAppInput } from './inputType'
import type { Output } from './rx'
import { getNextId } from '../flows/flowStateUtils'

export function allItems(
	grid: GridItem[],
	subgrids: Record<string, GridItem[]> | undefined
): GridItem[] {
	if (subgrids == undefined) {
		return grid
	}
	return [...grid, ...Object.values(subgrids).flat()]
}

export function allItemsWithParent(
	grid: GridItem[],
	subgrids: Record<string, GridItem[]> | undefined
): [GridItem, string | undefined][] {
	const items: [GridItem, string | undefined][] = grid.map((item) => [item, undefined])
	if (subgrids == undefined) {
		return items
	}
	return [
		...items,
		...Object.entries(subgrids).flatMap(([k, v]) => v.map((g) => [g, k] as [GridItem, string]))
	]
}

export async function loadSchema(
	workspace: string,
	path: string,
	runType: 'script' | 'flow' | 'hubscript'
): Promise<Schema> {
	if (runType === 'script') {
		const script = await ScriptService.getScriptByPath({
			workspace,
			path
		})

		return script.schema
	} else if (runType === 'flow') {
		const flow = await FlowService.getFlowByPath({
			workspace,
			path
		})

		return flow.schema
	} else {
		const script = await ScriptService.getHubScriptByPath({
			path
		})
		if (
			script.schema == undefined ||
			Object.keys(script.schema).length == 0 ||
			typeof script.schema != 'object'
		) {
			script.schema = emptySchema()
		}

		await inferArgs(script.language, script.content, script.schema)
		return script.schema
	}
}

export function schemaToInputsSpec(
	schema: Schema,
	defaultUserInput: boolean
): Record<string, StaticAppInput> {
	if (schema?.properties == undefined) {
		return {}
	}
	return Object.keys(schema.properties).reduce((accu, key) => {
		const property = schema.properties[key]

		accu[key] = {
			type: defaultUserInput && !property.format?.startsWith('resource-') ? 'user' : 'static',
			value: property.default,
			fieldType: property.type,
			format: property.format
		}

		return accu
	}, {})
}

export function accessPropertyByPath<T>(object: T, path: string): T | undefined {
	// convert indexes to properties
	path = path.replace(/\[(\w+)\]/g, '.$1')
	// strip a leading dot
	path = path.replace(/^\./, '')

	let a = path.split('.')

	for (let i = 0, depth = a.length; i < depth; ++i) {
		let key = a[i]
		if (object[key]) {
			object = object[key]
		} else {
			// Value not found
			return
		}
	}
	return object
}

export function fieldTypeToTsType(inputType: InputType): string {
	switch (inputType) {
		case 'number':
			return 'number'
		case 'boolean':
			return 'boolean'
		case 'object':
			return 'object'
		case 'array':
			return 'array'
		case 'any':
			return 'any'
		default:
			return 'string'
	}
}

export function isScriptByNameDefined(appInput: AppInput | undefined): boolean {
	if (!appInput) {
		return false
	}

	if (appInput.type === 'runnable' && appInput.runnable?.type == 'runnableByName') {
		return appInput.runnable?.name != undefined
	}

	return false
}

export function isScriptByPathDefined(appInput: AppInput | undefined): boolean {
	if (!appInput) {
		return false
	}

	if (appInput.type === 'runnable' && appInput.runnable?.type == 'runnableByPath') {
		return Boolean(appInput.runnable?.path)
	}

	return false
}

export function clearResultAppInput(appInput: ResultAppInput): ResultAppInput {
	appInput.runnable = undefined
	if (Object.keys(appInput.fields ?? {}).length > 0) {
		appInput.fields = {}
	}
	return appInput
}

export function toStatic(
	app: App,
	staticExporter: Record<string, () => any>,
	summary: string
): { app: App; summary: string } {
	const newApp: App = JSON.parse(JSON.stringify(app))
	newApp.grid.forEach((x) => {
		let c: AppComponent = x.data
		if (c.componentInput?.type == 'runnable') {
			c.componentInput.value = staticExporter[x.id]()
		}
	})
	return { app: newApp, summary }
}

export function buildExtraLib(
	components: Record<string, Record<string, Output<any>>>,
	idToExclude: string,
	hasRows: boolean
): string {
	return (
		Object.entries(components)
			.filter(([k, v]) => k != idToExclude)
			.map(([k, v]) => [k, Object.fromEntries(Object.entries(v).map(([k, v]) => [k, v.peak()]))])
			.map(
				([k, v]) => `

declare const ${k} = ${JSON.stringify(v)};

`
			)
			.join('\n') + (hasRows ? 'declare const row: Record<string, any>;' : '')
	)
}

export function getAllScriptNames(app: App): string[] {
	const names = app.grid.reduce((acc, gridItem: GridItem) => {
		const { componentInput } = gridItem.data

		if (
			componentInput?.type === 'runnable' &&
			componentInput?.runnable?.type === 'runnableByName'
		) {
			acc.push(componentInput.runnable.name)
		}

		if (componentInput?.type === 'tablecomponent') {
			componentInput.actionButtons.forEach((actionButton) => {
				if (actionButton.componentInput?.type === 'runnable') {
					if (actionButton.componentInput.runnable?.type === 'runnableByName') {
						acc.push(actionButton.componentInput.runnable.name)
					}
				}
			})
		}

		return acc
	}, [] as string[])

	const unusedNames = app.unusedInlineScripts.map((x) => x.name)
	const backgroundNames = app.hiddenInlineScripts?.map((x) => x.name) ?? []

	return [...names, ...unusedNames, ...backgroundNames]
}

function clearAndUpper(text: string) {
	return text.replace(/-/, '').toUpperCase()
}

export function toPascalCase(text: string) {
	return text.replace(/(^\w|-\w)/g, clearAndUpper)
}

export function toKebabCase(text: string) {
	return text.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase()
}

export function findParent(root: GridItem[], id: string): GridItem | undefined {
	for (const a of root) {
		if (a.id === id) {
			return a
		}

		if (a.data.subGrids) {
			// Recursively search the sub-grids
			for (const subGrid of a.data.subGrids) {
				const result = findParent(subGrid, id)
				if (result) {
					return result
				}
			}
		}
	}

	return undefined
}

export function insertNewGridItem(
	root: GridItem[],
	id: string,
	subGridIndex: number,
	newId: string,
	data: AppComponent
): GridItem[] {
	const parentA = findParent(root, id)

	if (!parentA) {
		throw new Error(`Parent A object with ID ${id} not found.`)
	}

	const subGrid = parentA.data.subGrids[subGridIndex]

	if (!subGrid) {
		throw new Error(`Sub-grid with index ${subGridIndex} not found.`)
	}

	const newItem = createNewGridItem(subGrid ?? [], newId, data)
	subGrid.push(newItem)
	return root
}

// The grid is needed to find a space for the new component
export function createNewGridItem(grid: GridItem[], id: string, data: AppComponent): GridItem {
	const appComponent = data

	appComponent.id = id

	const newComponent = {
		fixed: false,
		resizable: true,
		draggable: true,
		customDragger: false,
		customResizer: false,
		x: 0,
		y: 0
	}

	let newData: AppComponent = JSON.parse(JSON.stringify(appComponent))

	const newItem: GridItem = {
		data: newData,
		id: id
	}

	gridColumns.forEach((column) => {
		const rec = getRecommendedDimensionsByComponent(appComponent.type, column)

		newItem[column] = {
			...newComponent,
			min: { w: 1, h: 1 },
			max: { w: column, h: 100 },
			w: rec.w,
			h: rec.h
		}
		const position = gridHelp.findSpace(newItem, grid, column) as { x: number; y: number }
		newItem[column] = { ...newItem[column], ...position }
	})

	return newItem
}

function recursiveGetIds(gridItem: GridItem): string[] {
	const subGrids = gridItem.data.subGrids ?? []
	const subGridIds = subGrids
		.map((subGrid: GridItem[]) => subGrid.map(recursiveGetIds))
		.flat(Infinity)
	return [gridItem.data.id, ...subGridIds]
}

export function getNextGridItemId(grid: GridItem[] = []): string {
	const gridItemIds = grid.map(recursiveGetIds).flat()
	const id = getNextId(gridItemIds)
	return id
}

export function concatCustomCss<T extends string = string>(
	appCss?: ComponentCustomCSS<T>,
	componentCss?: ComponentCustomCSS<T>
): Record<T, ComponentCssProperty> | undefined {
	if (!componentCss) return undefined

	const customStyle = {} as Record<T, ComponentCssProperty>
	Object.keys(componentCss).forEach((key) => {
		const k = key as keyof typeof componentCss

		// This is the general style of the component type
		const appStyle = appCss?.[k]?.style?.trim() || ''
		const appEnding = appStyle?.endsWith(';') || !appStyle ? ' ' : '; '

		// This is the custom style of the component instance
		const compStyle = componentCss[k]?.style?.trim() || ''
		const compEnding = compStyle?.endsWith(';') || !compStyle ? ' ' : ';'

		customStyle[k] = {
			style: (appStyle + appEnding + compStyle + compEnding).trim(),
			class: twMerge(appCss?.[k]?.class, componentCss[k]?.class)
		}
	})

	return customStyle
}
