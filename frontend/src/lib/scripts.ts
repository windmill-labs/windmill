import { get } from 'svelte/store'
import { ScriptService } from './gen'
import { inferArgs } from './infer'
import { workspaceStore } from './stores'
import { emptySchema } from './utils'

export async function loadSchema(path: string) {
	if (path.startsWith('hub/')) {
		const code = await ScriptService.getHubScriptContentByPath({ path })
		const schema = emptySchema()
		await inferArgs('deno', code, schema)
		return schema
	} else {
		const script = await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path: path ?? ''
		})
		return script.schema
	}
}
