import { resource } from 'runed'
import { fromStore } from 'svelte/store'
import { WorkspaceService } from '$lib/gen'
import { copilotInfo, copilotWorkspace } from '$lib/aiStore'

/**
 * Whether a workspace hid the AI assistant (`ai_config.copilot_disabled`), read for that workspace
 * directly: `copilotInfo` holds whichever workspace loaded last, so it answers only when it was
 * loaded for this very workspace. `hidden` is `undefined` until a verdict for the workspace asked
 * about lands; runed keeps the previous `current` while a new source loads, so each verdict is
 * tagged with its workspace. A failed read counts as not hidden, as an unloaded config does
 * everywhere else.
 */
export function useWorkspaceAiHidden(workspace: () => string | undefined) {
	const loadedInfo = fromStore(copilotInfo)
	const loadedWorkspace = fromStore(copilotWorkspace)
	const verdict = resource(workspace, async (ws, _prev, { signal }) => {
		if (!ws) return { workspace: ws, hidden: false }
		let hidden = false
		try {
			hidden = (await WorkspaceService.getCopilotInfo({ workspace: ws })).copilot_disabled === true
		} catch (e) {
			console.error(`Failed to read the AI config of workspace ${ws}:`, e)
		}
		// The generated client can't take an abort signal, so drop a superseded response here.
		if (signal.aborted) throw new DOMException('superseded', 'AbortError')
		return { workspace: ws, hidden }
	})
	const hidden = $derived.by(() => {
		const ws = workspace()
		const current = verdict.current
		if (current && current.workspace === ws) return current.hidden
		return ws && loadedWorkspace.current === ws ? loadedInfo.current.workspaceDisabled : undefined
	})
	return {
		get hidden(): boolean | undefined {
			return hidden
		},
		/** Whether any verdict has landed yet, for any workspace. */
		get settledOnce(): boolean {
			return verdict.current !== undefined
		}
	}
}
