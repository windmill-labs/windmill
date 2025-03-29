import type { Schema } from './common'
import { AppService, FlowService, type Flow, type Script } from './gen'
import { encodeState } from './utils'
import rawHubPaths from './hubPaths.json?raw'
import {
	replacePlaceholderForSignatureScriptTemplate,
	SIGNATURE_TEMPLATE_FLOW_HUB_ID,
	SIGNATURE_TEMPLATE_SCRIPT_HUB_PATH
} from './components/triggers/http/utils'

export function scriptToHubUrl(
	content: string,
	summary: string,
	description: string,
	kind: Script['kind'],
	language: Script['language'],
	schema: Schema | any,
	lock: string | undefined,
	hubBaseUrl: string
): URL {
	const url = new URL(hubBaseUrl + '/scripts/add')
	url.hash = encodeState({ content, summary, description, kind, language, schema, lock })

	return url
}

export const HubScript = {
	SIGNATURE_TEMPLATE: SIGNATURE_TEMPLATE_SCRIPT_HUB_PATH
} as const

export const HubFlow = {
	SIGNATURE_TEMPLATE: SIGNATURE_TEMPLATE_FLOW_HUB_ID
} as const

export function replaceScriptPlaceholderWithItsValues(id: string, content: string) {
	switch (id) {
		case HubScript.SIGNATURE_TEMPLATE:
		case HubFlow.SIGNATURE_TEMPLATE:
			return replacePlaceholderForSignatureScriptTemplate(content)
		default:
			return content
	}
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

type HubPaths = {
	gitSync: string
	gitSyncTest: string
	slackErrorHandler: string
	slackRecoveryHandler: string
	slackSuccessHandler: string
	slackReport: string
	discordReport: string
	smtpReport: string
	teamsErrorHandler: string
	teamsRecoveryHandler: string
	teamsSuccessHandler: string
}

export const hubPaths = JSON.parse(rawHubPaths) as HubPaths
