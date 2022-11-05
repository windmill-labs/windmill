import type { Schema } from '$lib/common'
import type { App, InputsSpec } from '../../types'

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
