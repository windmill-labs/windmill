import type { InputsSpec } from './types'
import type { Schema } from '$lib/common'

import { FlowService, ScriptService } from '$lib/gen'
import {
	faBarChart,
	faDisplay,
	faFile,
	faMobileScreenButton,
	faPieChart
} from '@fortawesome/free-solid-svg-icons'

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
	}
}
