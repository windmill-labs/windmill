import { WorkspaceService } from '$lib/gen'
import { copilotWorkspace, setCopilotInfo } from '$lib/aiStore'
import { workspaceAIClients } from './lib'
import { writable } from 'svelte/store'

// The workspace of the most recent loadCopilot *request*, set synchronously before the
// await — as opposed to `copilotWorkspace`, which only updates once a load resolves. A
// background refresh (e.g. free-tier usage) compares against this so it can't supersede an
// in-flight load for a newer workspace (which would otherwise win the token and restore
// stale state).
export const copilotWorkspaceRequested = writable<string | undefined>(undefined)

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
// instead of firing one GET each. Only ever hold the newest request here: an older one
// has lost the token, so handing it back would leave its workspace never applied.
let inFlight: { workspace: string; promise: Promise<void> } | undefined

export function loadCopilot(workspace: string): Promise<void> {
	copilotWorkspaceRequested.set(workspace)
	if (inFlight?.workspace === workspace) {
		return inFlight.promise
	}
	const promise = fetchAndApply(workspace, ++loadCopilotToken).finally(() => {
		if (inFlight?.promise === promise) {
			inFlight = undefined
		}
	})
	inFlight = { workspace, promise }
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
