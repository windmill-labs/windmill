// Dedup signature for the session flow preview's two-way sync (FlowEditorView).
//
// The inbound (draft → editor) and outbound (editor → draft) effects compare
// this signature to skip no-op work. It MUST include `summary` — otherwise a
// summary-only change produces an identical signature and never propagates or
// persists.
export function flowDraftSig(x: { value?: unknown; schema?: unknown; summary?: unknown }): string {
	return JSON.stringify({ value: x.value, schema: x.schema, summary: x.summary })
}
