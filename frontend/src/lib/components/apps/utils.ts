import type { App, TriggerablePolicy, InputsSpec } from './types'
import type { Schema } from '$lib/common'

import { FlowService, ScriptService } from '$lib/gen'

type Args = Record<string, any>

export function buildArgs(
	inputSpecs: InputsSpec,
	schema: Schema,
	includeHidden: boolean = false
): Args {
	const obj = Object.keys(schema.properties).reduce((acc, key) => {
		let input = inputSpecs[key]

		if (!input) {
			input = {
				type: 'static',
				value: '',
				visible: true
			}
		}

		if (input.type === 'static' && (input.visible || includeHidden)) {
			acc[key] = input.value
		}

		if (input.type === 'output') {
			acc[key] = input.defaultValue
		}

		if (input.type === 'user') {
			acc[key] = schema.properties[key].default
		}

		return acc
	}, {})

	return obj
}

export async function loadSchema(
	workspace: string,
	triggerable: TriggerablePolicy
): Promise<Schema> {
	if (triggerable?.type === 'script') {
		const script = await ScriptService.getScriptByPath({
			workspace,
			path: triggerable.path
		})

		return script.schema
	} else {
		const flow = await FlowService.getFlowByPath({
			workspace,
			path: triggerable.path
		})

		return flow.schema
	}
}

export function isPolicyDefined(app: App, componentId: string): boolean {
	return app.policy?.triggerables[componentId] !== undefined
}

export function schemaToInputsSpec(schema: Schema): InputsSpec {
	return Object.keys(schema.properties).reduce((accu, key) => {
		const property = schema.properties[key]
		accu[key] = {
			type: 'static',
			defaultValue: property.default,
			value: undefined,
			visible: true
		}
		return accu
	}, {})
}
