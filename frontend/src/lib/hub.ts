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
	lock: string | undefined
): URL {
	const url = new URL('https://hub.windmill.dev/scripts/add')

	url.searchParams.append('content', content)
	url.searchParams.append('summary', summary)
	url.searchParams.append('description', description)
	url.searchParams.append('kind', kind)
	url.searchParams.append('language', language)
	url.searchParams.append('schema', JSON.stringify(schema, null, 2))
	lock && url.searchParams.append('lockfile', lock)

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

export function flowToHubUrl(flow: Flow): URL {
	const url = new URL('https://hub.windmill.dev/flows/add')
	const openFlow = {
		value: flow.value,
		summary: flow.summary,
		description: flow.description,
		schema: flow.schema
	}
	url.searchParams.append('flow', encodeState(openFlow))
	return url
}

export function appToHubUrl(staticApp: any): URL {
	const url = new URL('https://hub.windmill.dev/apps/add')
	url.searchParams.append('app', encodeState(staticApp))
	return url
}
