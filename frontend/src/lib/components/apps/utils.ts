import type { Schema } from '$lib/common'

import { FlowService, ScriptService } from '$lib/gen'
import {
	faBarChart,
	faDisplay,
	faFile,
	faMobileScreenButton,
	faPieChart
} from '@fortawesome/free-solid-svg-icons'
import type { InputType } from 'zlib'
import type { InputsSpec } from './inputType'

export async function loadSchema(
	workspace: string,
	path: string,
	runType: 'script' | 'flow'
): Promise<Schema> {
	if (runType === 'script') {
		const script = await ScriptService.getScriptByPath({
			workspace,
			path
		})

		return script.schema
	} else {
		const flow = await FlowService.getFlowByPath({
			workspace,
			path
		})

		return flow.schema
	}
}

export function schemaToInputsSpec(schema: Schema): InputsSpec {
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

export const displayData = {
	displaycomponent: {
		name: 'Result',
		icon: faDisplay
	},
	textcomponent: {
		name: 'Text',
		icon: faFile
	},
	buttoncomponent: {
		name: 'Button',
		icon: faMobileScreenButton
	},
	piechartcomponent: {
		name: 'Pie chart',
		icon: faPieChart
	},
	barchartcomponent: {
		name: 'Bar chart',
		icon: faBarChart
	},
	tablecomponent: {
		name: 'Table',
		icon: faBarChart
	},
	checkboxcomponent: {
		name: 'Checkbox',
		icon: faBarChart
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

export function fieldTypeToTsType(InputType: InputType): string {
	switch (InputType) {
		case 'number':
			return 'number'
		case 'boolean':
			return 'boolean'
		case 'object':
			return 'object'
		default:
			return 'string'
	}
}
