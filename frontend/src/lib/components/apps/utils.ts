import type { Schema, SupportedLanguage } from '$lib/common'
import { FlowService, ScriptService } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { emptySchema } from '$lib/utils'
import { twMerge } from 'tailwind-merge'
import type { AppComponent } from './editor/component'
import type { AppInput, InputType, ResultAppInput, StaticAppInput } from './inputType'
import type { Output } from './rx'
import type {
	App,
	ComponentCssProperty,
	GridItem,
	HorizontalAlignment,
	VerticalAlignment
} from './types'

export const BG_PREFIX = 'bg_'

export function migrateApp(app: App) {
	app?.hiddenInlineScripts.forEach((x) => {
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
): Promise<{ schema: Schema; summary: string | undefined }> {
	if (runType === 'script') {
		const script = await ScriptService.getScriptByPath({
			workspace,
			path
		})

		return { schema: script.schema as any, summary: script.summary }
	} else if (runType === 'flow') {
		const flow = await FlowService.getFlowByPath({
			workspace,
			path
		})

		return { schema: flow.schema as any, summary: flow.summary }
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

		await inferArgs(script.language as SupportedLanguage, script.content, script.schema)
		return { schema: script.schema, summary: script.summary }
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
declare function recompute(id: string): void;
declare function getAgGrid(id: string): {api: any, columnApi: any} | undefined;
declare function setValue(id: string, value: any): void;
declare function setSelectedIndex(id: string, index: number): void;
`
		: ''
}
declare const state: ${JSON.stringify(state)};
declare const iter: {index: number, value: any};
`
}

export function getAllScriptNames(app: App): string[] {
	const names = allItems(app.grid, app?.subgrids).reduce((acc, gridItem: GridItem) => {
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
	return text.replace(/[A-Z]+(?![a-z])|[A-Z]/g, ($, ofs) => (ofs ? '-' : '') + $.toLowerCase())
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
	if (source.startsWith('data:') || !source.includes(',')) {
		return source
	} else {
		return `data:application/octet-stream;base64,${source}`
	}
}
