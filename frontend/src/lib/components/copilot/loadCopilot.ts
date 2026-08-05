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
export async function loadCopilot(workspace: string) {
	const token = ++loadCopilotToken
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
