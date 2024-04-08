import type { Schema } from './common'
import { AppService, FlowService, type Flow, type Script } from './gen'
import { encodeState } from './utils'

export function scriptToHubUrl(
	content: string,
	summary: string,
	description: string,
	kind: Script.kind,
	language: Script.language,
	schema: Schema | any,
	lock: string | undefined,
	hubBaseUrl: string
): URL {
	const url = new URL(hubBaseUrl + '/scripts/add')
	url.hash = encodeState({ content, summary, description, kind, language, schema, lock })

	return url
}

export async function loadHubFlows() {
	try {
		const flows = (await FlowService.listHubFlows()).flows ?? []
		const processed = flows.sort((a, b) => b.votes - a.votes)
		return processed
	} catch {
		console.error('Hub is not available')
	}
}

export async function loadHubApps() {
	try {
		const apps = (await AppService.listHubApps()).apps ?? []
		const processed = apps.sort((a, b) => b.votes - a.votes)
		return processed
	} catch {
		console.error('Hub is not available')
	}
}

export function flowToHubUrl(flow: Flow, hubBaseUrl: string): URL {
	const url = new URL(hubBaseUrl + '/flows/add')
	const openFlow = {
		value: flow.value,
		summary: flow.summary,
		description: flow.description,
		schema: flow.schema
	}
	url.searchParams.append('flow', encodeState(openFlow))
	return url
}

export function appToHubUrl(staticApp: any, hubBaseUrl: string): URL {
	const url = new URL(hubBaseUrl + '/apps/add')
	url.searchParams.append('app', encodeState(staticApp))
	return url
}
