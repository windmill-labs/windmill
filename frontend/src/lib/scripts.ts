import { get } from 'svelte/store'
import type { Schema, SupportedLanguage } from './common'
import { FlowService, Script, ScriptService } from './gen'
import { inferArgs } from './infer'
import { workspaceStore, hubScripts } from './stores'
import { emptySchema } from './utils'

export function scriptLangToEditorLang(lang: Script.language) {
	if (lang == 'deno') {
		return 'typescript'
	} else if (lang == 'bun') {
		return 'typescript'
	} else if (lang == 'nativets') {
		return 'typescript'
	} else if (lang == 'graphql') {
		return 'typescript'
	} else if (lang == 'postgresql') {
		return 'sql'
	} else if (lang == 'mysql') {
		return 'sql'
	} else if (lang == 'python3') {
		return 'python'
	} else if (lang == 'bash') {
		return 'shell'
	} else {
		return lang
	}
}

export async function loadSchema(path: string, hash?: string): Promise<Schema> {
	if (path.startsWith('hub/')) {
		const { content, language, schema } = await ScriptService.getHubScriptByPath({ path })
		if (language == 'deno') {
			const newSchema = emptySchema()
			await inferArgs('deno' as SupportedLanguage, content ?? '', newSchema)
			return newSchema
		} else {
			return schema ?? emptySchema()
		}
	} else if (hash) {
		const script = await ScriptService.getScriptByHash({
			workspace: get(workspaceStore)!,
			hash
		})
		return inferSchemaIfNecessary(script)
	} else {
		const script = await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path: path ?? ''
		})
		return inferSchemaIfNecessary(script)
	}
}

async function inferSchemaIfNecessary(script: Script) {
	if (script.schema) {
		return script.schema as any
	} else {
		const newSchema = emptySchema()
		await inferArgs(script.language, script.content ?? '', newSchema)
		return newSchema
	}
}

export async function loadSchemaFlow(path: string): Promise<Schema> {
	const flow = await FlowService.getFlowByPath({
		workspace: get(workspaceStore)!,
		path: path ?? ''
	})
	return flow.schema as any
}

export function scriptPathToHref(path: string): string {
	if (path.startsWith('hub/')) {
		return 'https://hub.windmill.dev/from_version/' + path.substring(4)
	} else {
		return `/scripts/get/${path}?workspace=${get(workspaceStore)}`
	}
}

export async function getScriptByPath(path: string): Promise<{
	content: string
	language: SupportedLanguage
	schema: any
	description: string
	tag: string | undefined
}> {
	if (path.startsWith('hub/')) {
		const { content, language, schema } = await ScriptService.getHubScriptByPath({ path })

		return {
			content,
			language: language as SupportedLanguage,
			schema,
			description: '',
			tag: undefined
		}
	} else {
		const script = await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path: path ?? ''
		})
		return {
			content: script.content,
			language: script.language,
			schema: script.schema,
			description: script.description,
			tag: script.tag
		}
	}
}

export async function getLatestHashForScript(path: string): Promise<string> {
	const script = await ScriptService.getScriptByPath({
		workspace: get(workspaceStore)!,
		path: path ?? ''
	})
	return script.hash
}

export async function loadHubScripts() {
	try {
		const scripts = (await ScriptService.listHubScripts()).asks ?? []
		const processed = scripts
			.map((x) => ({
				path: `hub/${x.id}/${x.app}/${x.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
				summary: `${x.summary} (${x.app})`,
				approved: x.approved,
				kind: x.kind,
				app: x.app,
				views: x.views,
				votes: x.votes,
				ask_id: x.ask_id
			}))
			.sort((a, b) => b.views - a.views)
		hubScripts.set(processed)
	} catch {
		console.error('Hub is not available')
	}
}
