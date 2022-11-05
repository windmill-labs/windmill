import type { App, AppInputTransform, InputsSpec } from './types'
import type { Schema } from '$lib/common'

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

export function isAppInputTransformHidden(input: AppInputTransform): boolean {
	return (
		(input.type === 'static' && input.visible === false) ||
		(input.type === 'output' && input.visible === false)
	)
}
