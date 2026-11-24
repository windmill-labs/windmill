import {
	copilotInfo,
	copilotWorkspace,
	loadWorkspaceCopilotInfo,
	setCopilotInfo
} from '$lib/aiStore'
import { setAiClientWorkspace } from './aiClientWorkspace'

// Lives here, not in $lib/aiStore, purely so that module needs no import of the AI
// client — it is the one thing that wanted both. Moving it back recreates the
// aiStore -> copilot/lib -> copilot/chat/shared -> aiStore cycle.

// The tab's copy is global, so loads for two workspaces race on a fast switch: only the workspace
// asked for last is applied, whichever load resolves last.
let requested: string | undefined

/** Loads `workspace`'s AI config into the tab's copy. For the page's workspace only: a session
 * reads its own from the per-workspace cache. */
export async function loadCopilot(workspace: string): Promise<void> {
	requested = workspace
	setAiClientWorkspace(workspace)
	try {
		// Through the per-workspace cache: a session in this workspace then sends with the config
		// the tab shows.
		const info = await loadWorkspaceCopilotInfo(workspace, true)
		if (requested !== workspace) return
		copilotInfo.set(info)
		copilotWorkspace.set(workspace)
	} catch (err) {
		if (requested !== workspace) return
		setCopilotInfo({})
		copilotWorkspace.set(workspace)
		console.error('Could not get copilot info', err)
	}
}
