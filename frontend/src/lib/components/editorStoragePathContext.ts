import { getContext, setContext } from 'svelte'

// The path an editor's draft is stored under, the same key the live-editor draft
// registers. Published by whichever ancestor owns it (the session tab, or the
// full-page editor) because the parts that must say which item an editor is open
// on sit below both, and the path they can see themselves is the renamed one.

const KEY = 'EditorStoragePath'

/** `undefined` where the editor has no stored draft to speak of (a drawer mount). */
export type EditorStoragePath = () => string | undefined

export function setEditorStoragePath(storagePath: EditorStoragePath): void {
	setContext(KEY, storagePath)
}

export function getEditorStoragePath(): EditorStoragePath | undefined {
	return getContext<EditorStoragePath | undefined>(KEY)
}
