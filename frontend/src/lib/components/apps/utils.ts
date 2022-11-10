import type { App, TriggerablePolicy, InputsSpec } from './types'
import type { Schema } from '$lib/common'

import { FlowService, ScriptService } from '$lib/gen'

type Args = Record<string, any>

export function buildArgs(
	inputSpecs: InputsSpec,
	schema: Schema,
	includeHidden: boolean = false
): Args {
	return Object.keys(schema.properties).reduce((acc, key) => {
		const input = inputSpecs[key]
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
