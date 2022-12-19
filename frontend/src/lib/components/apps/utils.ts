import type { Schema } from '$lib/common'
import { FlowService, ScriptService } from '$lib/gen'
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
	TextCursorInput,
	Type,
	ToggleLeft
} from 'lucide-svelte'
import type { AppInput, AppInputs, InputType } from './inputType'
import type { AppComponent, InlineScript } from './types'

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

		debugger

		return script.schema
	}
}

export function schemaToInputsSpec(schema: Schema): AppInputs {
	return Object.keys(schema.properties).reduce((accu, key) => {
		const property = schema.properties[key]

		accu[key] = {
			type: 'static',
			defaultValue: property.default,
			value: undefined,
			visible: true,
			fieldType: property.type
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
	piechartcomponent: {
		name: 'Pie chart',
		icon: PieChart
	},
	barchartcomponent: {
		name: 'Bar chart',
		icon: BarChart4
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
		name: 'Number input',
		icon: Binary
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
		return Boolean(appInput.runnable?.name)
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
