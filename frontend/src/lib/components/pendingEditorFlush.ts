// Every mounted `SimpleEditor`, so a caller can materialise what the user typed without
// knowing which editors a page contains — a drawer nests them through SchemaForm and
// ArgInput, so enumerating them from the container does not scale. Plain module rather
// than the editor component: importing that pulls Monaco's side-effect imports into every
// graph that reaches this, and the components around it defer Monaco deliberately.
const liveEditors = new Set<{ flushPendingChanges: () => void }>()

/** Register a mounted editor; the returned function deregisters it. */
export function registerPendingEditor(editor: { flushPendingChanges: () => void }): () => void {
	liveEditors.add(editor)
	return () => liveEditors.delete(editor)
}

/** Drain every mounted editor's debounced buffer. For code that must act on what is on
 * screen before leaving it — persisting a draft before routing to a session. */
export function flushAllPendingEditorChanges(): void {
	for (const editor of liveEditors) editor.flushPendingChanges()
}

// Editors whose current text does not parse. Their value never reaches the bound field, so
// a caller persisting "what is on screen" would save the last value that did parse and
// leave without it. Registered by the editors that parse, not by the ones that only hold text.
const unparseable = new Set<object>()

/** Mark or clear this editor as holding text that does not parse. */
export function setEditorUnparseable(key: object, invalid: boolean): void {
	if (invalid) unparseable.add(key)
	else unparseable.delete(key)
}

/** Whether any editor on screen holds text that cannot be persisted as written. Registry-
 * wide rather than per-item: the editors that parse are nested arbitrarily deep and none
 * of them knows which draft it belongs to. */
export function anyEditorUnparseable(): boolean {
	return unparseable.size > 0
}
