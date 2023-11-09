import { get } from 'svelte/store'
import type { Schema, SupportedLanguage } from './common'
import { FlowService, Script, ScriptService } from './gen'
import { workspaceStore } from './stores'

export function scriptLangToEditorLang(lang: Script.language) {
	if (lang == 'deno') {
		return 'typescript'
	} else if (lang == 'bun') {
		return 'typescript'
	} else if (lang == 'nativets') {
		return 'typescript'
		// } else if (lang == 'graphql') {
		// 	return 'typescript'
	} else if (lang == 'postgresql') {
		return 'sql'
	} else if (lang == 'mysql') {
		return 'sql'
	} else if (lang == 'bigquery') {
		return 'sql'
	} else if (lang == 'snowflake') {
		return 'sql'
	} else if (lang == 'mssql') {
		return 'sql'
	} else if (lang == 'python3') {
		return 'python'
	} else if (lang == 'bash') {
		return 'shell'
	} else if (lang == 'powershell') {
		return 'powershell'
	} else if (lang == 'graphql') {
		return 'graphql'
	} else {
		return lang
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
	concurrent_limit: number | undefined
	concurrency_time_window_s: number | undefined
}> {
	if (path.startsWith('hub/')) {
		const { content, language, schema } = await ScriptService.getHubScriptByPath({ path })

		return {
			content,
			language: language as SupportedLanguage,
			schema,
			description: '',
			tag: undefined,
			concurrent_limit: undefined,
			concurrency_time_window_s: undefined
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
			tag: script.tag,
			concurrent_limit: script.concurrent_limit,
			concurrency_time_window_s: script.concurrency_time_window_s
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
