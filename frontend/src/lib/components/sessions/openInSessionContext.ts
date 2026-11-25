import { getContext, setContext, tick } from 'svelte'
import type { OpenInSessionSource } from './OpenInSessionButton.svelte'

// The "Open in AI session" hand-off, published by the component that owns the
// item being edited (FlowBuilder, RawAppEditor) for AI entry points too deep in
// the tree to be handed it as a prop — the inline code editor's toolbar sits
// four levels below the builder, behind a recursive module wrapper.

const KEY = 'OpenInSessionHandoff'

export type OpenInSessionHandoff = {
	/** The editor's hand-off, opening on `moduleId` when it addresses its parts
	 * (a flow step). `undefined` while the item has no path to open yet. */
	source: (opts?: { moduleId?: string }) => OpenInSessionSource | undefined
}

export function setOpenInSessionHandoff(handoff: OpenInSessionHandoff): void {
	setContext(KEY, handoff)
}

export function getOpenInSessionHandoff(): OpenInSessionHandoff | undefined {
	return getContext<OpenInSessionHandoff | undefined>(KEY)
}

/** The hand-off with an editor's in-flight keystrokes materialized first: the session loads the
 * item from its draft, which would otherwise miss the last (sub-second) edits. */
export function withEditorFlush(
	source: OpenInSessionSource,
	flushEditor: () => void
): OpenInSessionSource {
	return {
		...source,
		beforeOpen: async () => {
			flushEditor()
			// The flush lands in the draft store through an effect; let it run
			// before the hand-off persists that store.
			await tick()
			await source.beforeOpen?.()
		}
	}
}
