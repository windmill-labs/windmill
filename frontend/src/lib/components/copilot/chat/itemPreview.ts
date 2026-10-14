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
	/** Which side of the item to show. A card on a write-tool result means the
	 * draft the tool just wrote, so it asks for the editor; a path the model
	 * mentioned in prose means the item as it stands, so it asks for the
	 * deployed page. Declared per call site because only the call site knows
	 * which of the two it is. */
	mode: 'edit' | 'view'
}

/** Build the action a preview card or path link dispatches from its (kind, path). */
export function openItemPreviewAction(
	kind: PreviewCardKind,
	path: string,
	mode: 'edit' | 'view' = 'edit'
): OpenItemPreviewAction {
	return {
		// The side is part of the identity: the same path can be offered both ways
		// in one transcript, and a shared id would collapse them into one action.
		id: `open-item-preview:${kind}:${mode}:${path}`,
		type: 'open_item_preview',
		label:
			mode === 'view'
				? `Open ${kind === 'raw_app' ? 'app' : kind}`
				: `Open ${kind === 'raw_app' ? 'app' : kind} preview`,
		previewKind: kind,
		path,
		mode
	}
}
