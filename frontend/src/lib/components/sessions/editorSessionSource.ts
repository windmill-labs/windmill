import type { OpenInSessionSource } from './OpenInSessionButton.svelte'

/**
 * The "Open in AI session" source published by the editor the user is currently
 * on (flow / script / raw-app builder). The sidebar Workspace↔AI Sessions toggle
 * (`enterSessionMode`) reads it so switching into session mode from an editor
 * opens that item in the preview — the same handoff the in-editor "Open in AI
 * session" button performs — instead of dropping the user in an empty session.
 *
 * Editors register on mount and the returned cleanup drops the entry on
 * teardown; the session preview's own editor (marked by the `aiChatManager`
 * context) MUST NOT register, else the toggle would re-seed from the preview.
 *
 * A plain module ref, not `$state`: the only reader is `enterSessionMode`'s
 * click handler, which reads it imperatively at the moment of the toggle.
 */
let current: OpenInSessionSource | undefined

export function registerEditorSessionSource(source: OpenInSessionSource | undefined): () => void {
	current = source
	return () => {
		// Only clear if we're still the active registration — a newer editor may
		// have taken over before this one tore down.
		if (current === source) current = undefined
	}
}

export function getEditorSessionSource(): OpenInSessionSource | undefined {
	return current
}
