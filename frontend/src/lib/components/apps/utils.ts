import type { Schema } from '$lib/common'

import { twMerge } from 'tailwind-merge'
import { type AppComponent } from './editor/component'
import type { AppInput, InputType, ResultAppInput, StaticAppInput } from './inputType'
import type { Output } from './rx'
import type {
	App,
	ComponentCssProperty,
	GridItem,
	HorizontalAlignment,
	VerticalAlignment
} from './types'
import { gridColumns } from './gridUtils'

export const BG_PREFIX = 'bg_'

export function migrateApp(app: App) {
	;(app?.hiddenInlineScripts ?? []).forEach((x) => {
		if (x.type == undefined) {
			//@ts-ignore
			x.type = 'runnableByName'
		}
		//TODO: remove after migration is done
		if (x.doNotRecomputeOnInputChanged != undefined) {
			x.recomputeOnInputChanged = !x.doNotRecomputeOnInputChanged
			x.doNotRecomputeOnInputChanged = undefined
		}
	})

	allItems(app.grid, app.subgrids).forEach((x) => {
		gridColumns.forEach((column: number) => {
			if (x?.[column]?.fullHeight === undefined) {
				x[column].fullHeight = false
			}
		})
	})
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
			type: defaultUserInput ? 'user' : 'static',
			value: property.default,
			fieldType: property.type,
			format: property.format
		}

		if (Array.isArray(property.enum)) {
			accu[key] = {
				...accu[key],
				selectOptions: property.enum,
				fieldType: 'select'
			}
		}

		return accu
	}, {})
}

export function accessPropertyByPath<T>(object: T, path: string): any {
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
		case 'integer':
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
	allItems(newApp.grid, newApp.subgrids).forEach((x) => {
		let c: AppComponent = x.data
		if (c.componentInput?.type == 'runnable') {
			c.componentInput.value = staticExporter[x.id]()
		}
	})

	newApp.hiddenInlineScripts?.forEach((x, i) => {
		x.noBackendValue = staticExporter[BG_PREFIX + i]()
	})

	return { app: newApp, summary }
}

// function hasCycle(path, obj, seen = new Set()) {
// 	if (obj && typeof obj === 'object') {
// 		if (seen.has(obj)) {
// 			console.log('cycle detected', path)
// 			return true // cycle detected
// 		}
// 		seen.add(obj)
// 		for (const key in obj) {
// 			if (hasCycle(path + '.' + key, obj[key], seen)) {
// 				return true
// 			}
// 		}
// 		seen.delete(obj) // backtrack for other branches
// 	}
// 	return false
// }

export function deepCloneWithFunctions(obj) {
	if (obj === null || typeof obj !== 'object') return obj
	if (typeof obj === 'function') return obj

	if (Array.isArray(obj)) {
		return obj.map(deepCloneWithFunctions)
	}

	const cloned = {}
	for (const key in obj) {
		cloned[key] = deepCloneWithFunctions(obj[key])
	}
	return cloned
}

export function buildExtraLib(
	components: Record<string, Record<string, Output<any>>>,
	idToExclude: string,
	state: Record<string, any>,
	goto: boolean
): string {
	// console.log(
	// 	Object.entries(components).map(([k, v]) => [
	// 		k,
	// 		Object.values(v).map((x) => x.peak()),
	// 		hasCycle(
	// 			k,
	// 			Object.values(v).map((x) => x.peak())
	// 		)
	// 	])
	// )
	const cs = Object.entries(components)
		.filter(([k, v]) => k != idToExclude && k != 'state')
		.map(([k, v]) => [k, Object.fromEntries(Object.entries(v).map(([k, v]) => [k, v.peak()]))])
		.map(
			([k, v]) => `declare const ${k}: WidenRo<${JSON.stringify(v)}>;
`
		)
		.join('\n')

	return `

type Widen<T> = T extends string
  ? string
  : T extends number
  ? number
  : T extends boolean
  ? boolean
  : T extends Array<infer U>
  ? Array<Widen<U>>
  : T extends object
  ? Partial<{ [K in keyof T]: Widen<T[K]> } & {[key: string]: any}>
  : T;

type WidenRo<T> = T extends string
  ? string
  : T extends number
  ? number
  : T extends boolean
  ? boolean
  : T extends Array<infer U>
  ? Readonly<Array<WidenRo<U>>>
  : T extends object
  ? Readonly<{ [K in keyof T]: WidenRo<T[K]> }>
  : T;


/** The mutable state of the app */
declare const state: Widen<${JSON.stringify(state)}> & {[key: string]: any};

${cs}

${
	goto
		? `
/** open a window or tab
 * @param path path/url to go to
 * @param open in a tab?
*/
declare async function goto(path: string, newTab?: boolean): Promise<void>;

/** set tab
 * @param id component's id
 * @param index index of the tab to set
*/
declare function setTab(id: string, index: number): void;

/** recompute a component's runnable or background runnable
 * @param id component's id
*/
declare function recompute(id: string): void;

/** recompute all components runnables and background runnables
 * Make sure to disable the toggle "Run on start and app refresh" in the 
 * settings panel to avoid circular recomputes
*/
declare function globalRecompute(): void;

/** get the ag grid api from an AgGridTable
 * @param id component's id
*/
declare function getAgGrid(id: string): {api: any, columnApi: any} | undefined;

/** set value of a component
 * @param id component's id
 * @param value value to set
 */
declare function setValue(id: string, value: any): void;

/** set selected index of a table
 * @param id component's id
 * @param index index to set
 */
declare function setSelectedIndex(id: string, index: number): void;

/** open a drawer or modal
  * @param id component's id
 */
declare function open(id: string): void;

/** close a drawer or modal
  * @param id component's id
 */
declare function close(id: string): void;


/** validate form field property 'key'
 * @param id component's id
 * @param key property's key to validate
 */
declare function validate(id: string, key: string): void;

/** validate form field property 'key'
 * @param id component's id
 * @param key property's key to validate
 */
declare function invalidate(id: string, key: string, error: string): void;

/** validate all form's properties
 * @param id component's id
 */
declare function validateAll(id: string): void;

/** Clear the files of a file input component
 * @param id component's id
 */
declare function clearFiles(id: string): void;

/**  Display a toast message
 * @param message message to display
 */
declare function showToast(message: string, error?: boolean): void;

/**
 * Wait for a job to finish
 * @param id job id
 * @returns the result of the job
 */
declare async function waitJob(id: string): Promise<any>;


/**
 * ask user resource on a UserResourceComponent
 */
declare async function askNewResource(id: string): void;

/**
 * Download a file from a url or base64 encoded string
 * @param input url, base64 encoded string, [dataUrl](https://developer.mozilla.org/en-US/docs/Web/URI/Schemes/data) or S3 object
 * @param [fileName] name of the file to download (optional)
 */
declare function downloadFile(input: string | {s3: string, storage?: string}, fileName?: string): void;

`
		: ''
}

/** The iterator within the context of a list */
declare const iter: {index: number, value: any};

/** The row within the context of a table */
declare const row: {index: number, value: Record<string, any>, disabled: boolean};

/** The file within the s3 file input */
declare const file: File | undefined;

/** The group fields within the context of a container's group */
declare const group: Record<string, any>;

/** The result within the context of a transformer */
declare const result: any;

`
}

export function getAllScriptNames(app: App): string[] {
	const names = (allItems(app.grid, app?.subgrids) ?? []).reduce((acc, gridItem: GridItem) => {
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

		if (
			gridItem.data.type === 'aggridcomponent' ||
			gridItem.data.type === 'aggridcomponentee' ||
			gridItem.data.type === 'dbexplorercomponent' ||
			gridItem.data.type === 'aggridinfinitecomponent' ||
			gridItem.data.type === 'aggridinfinitecomponentee'
		) {
			gridItem.data.actions?.forEach((actionButton) => {
				if (actionButton.componentInput?.type === 'runnable') {
					if (actionButton.componentInput.runnable?.type === 'runnableByName') {
						acc.push(actionButton.componentInput.runnable.name)
					}
				}
			})
		}

		if (gridItem.data.type === 'menucomponent') {
			gridItem.data.menuItems.forEach((menuItem) => {
				if (menuItem.componentInput?.type === 'runnable') {
					if (menuItem.componentInput.runnable?.type === 'runnableByName') {
						acc.push(menuItem.componentInput.runnable.name)
					}
				}
			})
		}

		return acc
	}, [] as string[])

	const unusedNames = app.unusedInlineScripts?.map((x) => x.name) ?? []
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
	return text.replace(/[A-Z]+(?![a-z])|[A-Z]/g, ($, ofs) => (ofs ? '-' : '') + $.toLowerCase())
}

export function initCss<T extends Record<string, ComponentCssProperty>>(
	appCss?: Record<string, ComponentCssProperty>,
	componentCss?: T
): T {
	if (!componentCss) return {} as T

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
					class: twMerge(appCss?.[key]?.class, v?.class),
					evalClass: appCss?.[key]?.evalClass || v?.evalClass
				}
			]
		})
	) as T
}

export function tailwindHorizontalAlignment(alignment?: HorizontalAlignment) {
	if (!alignment) return ''
	const classes: Record<HorizontalAlignment, string> = {
		left: 'justify-start',
		center: 'justify-center',
		right: 'justify-end'
	}
	return classes[alignment]
}

export function tailwindVerticalAlignment(alignment?: VerticalAlignment) {
	if (!alignment) return ''
	const classes: Record<VerticalAlignment, string> = {
		top: 'items-start',
		center: 'items-center',
		bottom: 'items-end'
	}
	return classes[alignment]
}

export const TailwindClassPatterns = {
	bg: /bg-(?:[^-\s]+-)?(?:[^-\s]+)/g,
	height: /(h-[^\s]+)/g,
	width: /(w-[^\s]+)/g
}

export function hasTailwindClass(classes: string | undefined, pattern: RegExp) {
	return Boolean(classes?.match(pattern))
}

export function transformBareBase64IfNecessary(source: string | undefined) {
	if (!source) {
		return source
	}
	if (/^(http|https):\/\//.test(source) || source.startsWith('/')) {
		return source
	}
	if (source.startsWith('data:') || source.includes(',')) {
		return source
	} else {
		return `data:application/octet-stream;base64,${source}`
	}
}

export function getImageDataURL(imageKind: string | undefined, image: string | undefined) {
	if (!imageKind || !image) {
		return null
	}

	switch (imageKind) {
		case 'png encoded as base64':
			return 'data:image/png;base64,' + image
		case 'jpeg encoded as base64':
			return 'data:image/jpeg;base64,' + image
		case 'svg encoded as base64':
			return 'data:image/svg+xml;base64,' + image
		default:
			return image
	}
}

export const ctxRegex = /^ctx\.(workspace|groups|username|email|author)$/
