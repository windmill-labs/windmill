import type { Schema } from '$lib/common'
import { FlowService, ScriptService } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { emptySchema, sendUserToast } from '$lib/utils'
import type { AppComponent, components } from './editor/component'
import type { App, ComponentCssProperty, GridItem } from './types'
import { twMerge } from 'tailwind-merge'
import type { AppInput, InputType, ResultAppInput, StaticAppInput } from './inputType'
import type { Output } from './rx'
import { get, type Writable } from 'svelte/store'
import { findGridItemParentGrid } from './editor/appUtils'

export function selectId(
	e: PointerEvent,
	id: string,
	selectedComponent: Writable<string[] | undefined>,
	app: App
) {
	if (e.shiftKey) {
		selectedComponent.update((old) => {
			if (old && old?.[0]) {
				if (findGridItemParentGrid(app, old[0]) != findGridItemParentGrid(app, id)) {
					sendUserToast('Cannot multi select items from different grids', true)
					return old
				}
			}
			if (old == undefined) {
				return [id]
			}
			if (old.includes(id)) {
				return old
			}
			return [...old, id]
		})
	} else {
		if (get(selectedComponent)?.includes(id)) {
			return
		} else {
			selectedComponent.set([id])
		}
	}
}

export function allItems(
	grid: GridItem[],
	subgrids: Record<string, GridItem[]> | undefined
): GridItem[] {
	if (subgrids == undefined) {
		return grid
	}
	return [...grid, ...Object.values(subgrids).flat()]
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
		if (object[key] != undefined) {
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
		case 'tab-select':
			return 'Tab'
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
	hasRows: boolean,
	state: Record<string, any>,
	goto: boolean
): string {
	const cs = Object.entries(components)
		.filter(([k, v]) => k != idToExclude)
		.map(([k, v]) => [k, Object.fromEntries(Object.entries(v).map(([k, v]) => [k, v.peak()]))])
		.map(
			([k, v]) => `declare const ${k}: ${JSON.stringify(v)};
`
		)
		.join('\n')

	return `${cs}
${hasRows ? 'declare const row: Record<string, any>;' : ''}
${
	goto
		? `declare async function goto(path: string, newTab?: boolean): Promise<void>;
declare function setTab(id: string, index: string): void;
`
		: ''
}
declare const state: ${JSON.stringify(state)};
`
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

		if (gridItem.data.type === 'tablecomponent') {
			gridItem.data.actionButtons.forEach((actionButton) => {
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

export function concatCustomCss<T extends Record<string, ComponentCssProperty>>(
	appCss?: Record<string, ComponentCssProperty>,
	componentCss?: T
): T | undefined {
	if (!componentCss) return undefined

	return Object.fromEntries(
		Object.entries(componentCss).map(([key, v]) => {
			// This is the general style of the component type
			const appStyle = appCss?.[key]?.style?.trim() || ''
			const appEnding = appStyle?.endsWith(';') || !appStyle ? ' ' : '; '

			// This is the custom style of the component instance
			const compStyle = v?.style?.trim() || ''
			const compEnding = compStyle?.endsWith(';') || !compStyle ? ' ' : ';'

			return [
				key,
				{
					style: (appStyle + appEnding + compStyle + compEnding).trim(),
					class: twMerge(appCss?.[key]?.class, v?.class)
				}
			]
		})
	) as T
}
