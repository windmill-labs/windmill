import { WorkspaceService } from '$lib/gen'
import { copilotWorkspace, setCopilotInfo } from '$lib/aiStore'
import { workspaceAIClients } from './lib'

// Lives here, not in $lib/aiStore, purely so that module needs no import of the AI
// client — it is the one thing that wanted both. Moving it back recreates the
// aiStore -> copilot/lib -> copilot/chat/shared -> aiStore cycle.

// copilotInfo/copilotSessionModel are global, so concurrent loads (e.g. a fast
// session switch between workspaces) race: an earlier call resolving last would
// clobber the active workspace's config. Apply only the most recent call's
// result via a monotonic token — last invocation wins regardless of resolution
// order. init() is synchronous so its ordering already matches.
let loadCopilotToken = 0

// Consumers load the config for the workspace they operate on rather than trusting an
// ancestor to have done it, and they mount together — so share the in-flight request
// instead of firing one GET each. Only the newest load applies its result, hence only
// the newest is shareable: a superseded workspace needs a fresh request to win again.
let inFlight: { workspace: string; token: number; promise: Promise<void> } | undefined

export function loadCopilot(workspace: string): Promise<void> {
	if (inFlight && inFlight.workspace === workspace && inFlight.token === loadCopilotToken) {
		return inFlight.promise
	}
	const token = ++loadCopilotToken
	const promise = fetchAndApply(workspace, token).finally(() => {
		if (inFlight?.token === token) {
			inFlight = undefined
		}
	})
	inFlight = { workspace, token, promise }
	return promise
}

async function fetchAndApply(workspace: string, token: number) {
	workspaceAIClients.init(workspace)
	try {
		const info = await WorkspaceService.getCopilotInfo({ workspace })
		if (token !== loadCopilotToken) return
		setCopilotInfo(info)
		copilotWorkspace.set(workspace)
	} catch (err) {
		if (token !== loadCopilotToken) return
		setCopilotInfo({})
		copilotWorkspace.set(workspace)
		console.error('Could not get copilot info', err)
	}
}
