import type { App, TriggerablePolicy, InputsSpec } from './types'
import type { Schema } from '$lib/common'

import { FlowService, ScriptService } from '$lib/gen'

type Args = Record<string, any>

export function buildArgs(
	inputSpecs: InputsSpec,
	triggerable: TriggerablePolicy,
	schema: Schema
): Args {
	return Object.entries(schema.properties).reduce((acc, [key, value]) => {
		const input = inputSpecs[key]
		if (input.type === 'static' && input.visible) {
			acc[key] = triggerable.staticFields[key]
		}

		if (input.type === 'output' && input.visible) {
			acc[key] = triggerable.staticFields[key]
		}

		if (input.type === 'user') {
			acc[key] = schema.properties[key].default
		}

		return acc
	}, {})
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

export function mergeArgs(defaultArgs: Args, args: Args): Args {
	return {
		...defaultArgs,
		...args
	}
}

export function isPolicyDefined(app: App, componentId: string): boolean {
	return app.policy?.triggerables[componentId] !== undefined
}

export function schemaToInputsSpec(schema: Schema | undefined): InputsSpec {
	const inputsSpec: InputsSpec = {}

	if (schema) {
		for (const [key, value] of Object.entries(schema.properties)) {
			inputsSpec[key] = {
				type: 'user',
				schemaProperty: value,
				defaultValue: value.default
			}
		}
	}

	return inputsSpec
}
