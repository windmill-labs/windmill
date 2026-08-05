import { AppService, FlowService } from './gen'
import hubPathsData from './hubPaths.json'
import {
	replacePlaceholderForSignatureScriptTemplate,
	SIGNATURE_TEMPLATE_FLOW_HUB_ID,
	SIGNATURE_TEMPLATE_SCRIPT_HUB_PATH
} from './components/triggers/http/utils'

export const DEFAULT_HUB_BASE_URL = 'https://hub.windmill.dev'
export const PRIVATE_HUB_MIN_VERSION = 10_000_000

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

type HubPaths = {
	gitSyncTest: string
	gitInitRepo: string
	slackErrorHandler: string
	slackRecoveryHandler: string
	slackSuccessHandler: string
	slackReport: string
	discordReport: string
	smtpReport: string
	teamsErrorHandler: string
	teamsRecoveryHandler: string
	teamsSuccessHandler: string
	emailErrorHandler: string
	cloneRepoToS3forGitRepoViewer: string
	appReport: string
}

export const hubPaths = hubPathsData as HubPaths
