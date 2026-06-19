// Dedup signature for the session flow preview's two-way sync (FlowEditorView).
//
// The inbound (draft → editor) and outbound (editor → draft) effects compare
// this signature to skip no-op work. It MUST include every top-level field the
// editor can change on its own — `summary`, `description`, and the path —
// otherwise a change to only that field produces an identical signature and
// never propagates or persists. FlowBuilder mirrors the Path widget's typed
// path into the flow's own `path` (the flow is parked at its `…/draft_<uuid>`
// storage key); without `path` here a rename moves no signature and the draft
// is never saved.
export function flowDraftSig(x: {
	value?: unknown
	schema?: unknown
	summary?: unknown
	description?: unknown
	path?: unknown
}): string {
	return JSON.stringify({
		value: x.value,
		schema: x.schema,
		summary: x.summary,
		description: x.description,
		path: x.path
	})
}
