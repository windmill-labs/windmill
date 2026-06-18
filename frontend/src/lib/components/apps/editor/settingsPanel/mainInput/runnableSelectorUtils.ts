import type { Schema } from '$lib/common'
import type { Runnable, StaticAppInput } from '$lib/components/apps/inputType'
import { schemaToInputsSpec } from '$lib/components/apps/utils'
import { defaultIfEmptyString } from '$lib/utils'

export type LoadedRunnableSchema = {
	schema: Schema
	summary: string | undefined
}

export function buildPathRunnableSelection(
	path: string,
	runType: 'script' | 'flow' | 'hubscript',
	loadedSchema: LoadedRunnableSchema,
	defaultUserInput: boolean,
	rawApps: boolean
): {
	runnable: Runnable
	fields: Record<string, StaticAppInput>
} {
	return {
		runnable: {
			type: 'path',
			path,
			runType,
			schema: loadedSchema.schema,
			name: defaultIfEmptyString(loadedSchema.summary, path)
		},
		fields: schemaToInputsSpec(loadedSchema.schema, defaultUserInput || rawApps)
	}
}
