import type { Flow, OpenFlow } from '$lib/gen'

// The "Open in AI session" content hand-off. Kept out of sessionRuntime so the
// editors that capture a seed (FlowBuilder) reach the helper without a runtime
// import of the session runtime's graph (chat manager → monaco).

/** An editor's live content, handed to a fresh session so its preview renders
 * without re-fetching what the page already holds. Consumed by
 * `seedEditorCell`. */
export type EditorSeed = {
	kind: 'flow'
	path: string
	/** The workspace the content was read from; the seed is dropped when the
	 * session ended up acting on another one (a fork), whose content differs. */
	workspace: string
	/** Typed as the editor holds it; the cell it seeds types the same loaded row
	 * as a `Flow` (see seedEditorCell). */
	flow: OpenFlow
	/** Per-module schemas and test results — the expensive half: rebuilding it
	 * costs one script fetch per path-referenced step (see initFlowState). */
	flowState: Record<string, any>
	saved: (Flow & { no_deployed?: boolean }) | undefined
}

/** Snapshot an editor's stores as an {@link EditorSeed}, detached from their
 * reactive state so the two editors can't alias one object. Returns undefined
 * for anything that won't clone, leaving the caller on the fetching path. */
export function captureEditorSeed(seed: EditorSeed): EditorSeed | undefined {
	try {
		return structuredClone($state.snapshot(seed)) as EditorSeed
	} catch (e) {
		console.error('Failed to capture editor seed', e)
		return undefined
	}
}
