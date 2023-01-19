import type { Schema } from '$lib/common'
import { FlowService, ScriptService } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { emptySchema } from '$lib/utils'
import {
	BarChart4,
	Binary,
	CircleDot,
	FormInput,
	Inspect,
	List,
	Monitor,
	PieChart,
	Play,
	Table2,
	Image,
	TextCursorInput,
	Type,
	Lock,
	Calendar,
	ToggleLeft,
	GripHorizontal,
	Code2,
	SlidersHorizontal,
	PlusSquare
} from 'lucide-svelte'
import type { AppInput, InputType, ResultAppInput, StaticAppInput } from './inputType'
import type { Output } from './rx'
import type { App, AppComponent, GridItem } from './types'

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

export const displayData: Record<AppComponent['type'], { name: string; icon: any }> = {
	displaycomponent: {
		name: 'Result',
		icon: Monitor
	},
	textcomponent: {
		name: 'Text',
		icon: Type
	},
	buttoncomponent: {
		name: 'Button',
		icon: Inspect
	},
	formcomponent: {
		name: 'Form',
		icon: FormInput
	},
	formbuttoncomponent: {
		name: 'Modal Form',
		icon: PlusSquare
	},
	piechartcomponent: {
		name: 'Pie Chart',
		icon: PieChart
	},
	barchartcomponent: {
		name: 'Bar/Line Chart',
		icon: BarChart4
	},
	htmlcomponent: {
		name: 'Html',
		icon: Code2
	},
	vegalitecomponent: {
		name: 'Vega Lite',
		icon: PieChart
	},
	timeseriescomponent: {
		name: 'Timeseries',
		icon: GripHorizontal
	},
	scatterchartcomponent: {
		name: 'Scatter Chart',
		icon: GripHorizontal
	},
	tablecomponent: {
		name: 'Table',
		icon: Table2
	},
	checkboxcomponent: {
		name: 'Toggle',
		icon: ToggleLeft
	},
	textinputcomponent: {
		name: 'Text input',
		icon: TextCursorInput
	},
	imagecomponent: {
		name: 'Image',
		icon: Image
	},
	inputcomponent: {
		name: 'Input',
		icon: FormInput
	},
	radiocomponent: {
		name: 'Radio button',
		icon: CircleDot
	},
	runformcomponent: {
		name: 'Run form',
		icon: Play
	},
	selectcomponent: {
		name: 'Select',
		icon: List
	},
	numberinputcomponent: {
		name: 'Number',
		icon: Binary
	},
	slidercomponent: {
		name: 'Slider',
		icon: SlidersHorizontal
	},
	passwordinputcomponent: {
		name: 'Password',
		icon: Lock
	},
	dateinputcomponent: {
		name: 'Date input',
		icon: Calendar
	}
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
