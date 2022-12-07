import { get } from 'svelte/store'
import type { Schema } from './common'
import { FlowService, ScriptService } from './gen'
import { inferArgs } from './infer'
import { workspaceStore } from './stores'
import { emptySchema } from './utils'

export async function loadSchema(path: string): Promise<Schema> {
	if (path.startsWith('hub/')) {
		const { content, language, schema } = await ScriptService.getHubScriptByPath({ path })
		if (language == 'deno') {
			const newSchema = emptySchema()
			await inferArgs('deno', content ?? '', newSchema)
			return newSchema
		} else {
			return schema ?? emptySchema()
		}
	} else {
		const script = await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path: path ?? ''
		})
		return script.schema
	}
}


export async function loadSchemaFlow(path: string): Promise<Schema> {
	const flow = await FlowService.getFlowByPath({
		workspace: get(workspaceStore)!,
		path: path ?? ''
	})
	return flow.schema

}
