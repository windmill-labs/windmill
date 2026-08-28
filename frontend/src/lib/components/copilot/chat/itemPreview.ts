// The session preview panel's action, kept out of `shared.ts` so the chat message render
// path can import it at runtime without pulling in that module's graph and risking the
// chunk cycles docs/frontend-import-cycles.md exists to prevent. Keep this file import-free.

/** Item kinds a session preview can host: the three live editors, which are also the
 * subset a write tool can land. */
export type PreviewCardKind = 'script' | 'flow' | 'raw_app'

// Dispatched by a preview card on a tool call that created or updated a workspace item,
// and by a path link in a chat message. Opens the item's live editor in the session side
// panel — or focuses the tab if it is already open. The handler is registered by the
// sessions page (the only surface with a preview panel).
export type OpenItemPreviewAction = {
	id: string
	type: 'open_item_preview'
	label: string
	previewKind: PreviewCardKind
	path: string
}

/** Build the action a preview card or path link dispatches from its (kind, path). */
export function openItemPreviewAction(kind: PreviewCardKind, path: string): OpenItemPreviewAction {
	return {
		id: `open-item-preview:${kind}:${path}`,
		type: 'open_item_preview',
		label: `Open ${kind === 'raw_app' ? 'app' : kind} preview`,
		previewKind: kind,
		path
	}
}
