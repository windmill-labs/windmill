import { get } from 'svelte/store'
import type { Schema, SupportedLanguage } from './common'
import { FlowService, type Script, ScriptService, ScheduleService } from './gen'
import { workspaceStore } from './stores'

export function scriptLangToEditorLang(lang: Script['language'] | undefined) {
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
	} else if (lang == 'php') {
		return 'php'
	} else if (lang == 'graphql') {
		return 'graphql'
	} else if (lang == undefined) {
		return 'typescript'
	} else {
		return lang
	}
}

export type ScriptSchedule = {
	summary: string | undefined
	args: Record<string, any>
	cron: string
	timezone: string
	enabled: boolean
}

// Load the schedule of a flow given its path and the workspace
export async function loadScriptSchedule(
	path: string,
	workspace: string
): Promise<ScriptSchedule | undefined> {
	const existsSchedule = await ScheduleService.existsSchedule({
		workspace,
		path
	})

	if (!existsSchedule) {
		return undefined
	}

	const schedule = await ScheduleService.getSchedule({
		workspace,
		path
	})

	return {
		summary: schedule.summary,
		enabled: schedule.enabled,
		cron: schedule.schedule,
		timezone: schedule.timezone,
		args: schedule.args ?? {}
	}
}

export async function loadSchemaFlow(path: string): Promise<Schema> {
	const flow = await FlowService.getFlowByPath({
		workspace: get(workspaceStore)!,
		path: path ?? ''
	})
	return flow.schema as any
}

export function scriptPathToHref(path: string, hubBaseUrl: string): string {
	if (path.startsWith('hub/')) {
		return hubBaseUrl + '/from_version/' + path.substring(4)
	} else {
		return `/scripts/get/${path}?workspace=${get(workspaceStore)}`
	}
}

const scriptLanguagesArray: [SupportedLanguage | 'docker', string][] = [
	['bun', 'TypeScript (Bun)'],
	['python3', 'Python'],
	['deno', 'TypeScript (Deno)'],
	['bash', 'Bash'],
	['go', 'Go'],
	['nativets', 'REST'],
	['postgresql', 'PostgreSQL'],
	['mysql', 'MySQL'],
	['bigquery', 'BigQuery'],
	['snowflake', 'Snowflake'],
	['mssql', 'MS SQL Server'],
	['graphql', 'GraphQL'],
	['powershell', 'PowerShell'],
	['php', 'PHP'],
	['docker', 'Docker']
]
export const defaultScriptLanguages = Object.fromEntries(scriptLanguagesArray)

export async function getScriptByPath(path: string): Promise<{
	content: string
	language: SupportedLanguage
	schema: any
	description: string
	tag: string | undefined
	concurrent_limit: number | undefined
	concurrency_time_window_s: number | undefined
	lock?: string
}> {
	if (path.startsWith('hub/')) {
		const { content, language, schema, lockfile } = await ScriptService.getHubScriptByPath({ path })

		return {
			content,
			language: language as SupportedLanguage,
			schema,
			description: '',
			tag: undefined,
			concurrent_limit: undefined,
			concurrency_time_window_s: undefined,
			lock: lockfile
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
			concurrency_time_window_s: script.concurrency_time_window_s,
			lock: script.lock
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
