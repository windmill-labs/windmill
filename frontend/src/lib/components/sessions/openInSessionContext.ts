import { getContext, onDestroy, setContext } from 'svelte'
import type { OpenInSessionSource } from './OpenInSessionButton.svelte'
import type { PreviewItemRoute } from './previewPaths'

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

// The hand-offs of every editor currently mounted. The navigation rail's "AI
// Sessions" switch sits above every page, out of reach of the context, and
// looks the editor of the item it is leaving up here instead.
const mounted = new Set<OpenInSessionHandoff>()

export function setOpenInSessionHandoff(handoff: OpenInSessionHandoff): void {
	setContext(KEY, handoff)
	onDestroy(registerMountedOpenInSessionHandoff(handoff))
}

/** Count `handoff` as mounted until the returned unregister runs. Split from
 * setOpenInSessionHandoff so the registry can be driven outside a component. */
export function registerMountedOpenInSessionHandoff(handoff: OpenInSessionHandoff): () => void {
	mounted.add(handoff)
	return () => {
		mounted.delete(handoff)
	}
}

export function getOpenInSessionHandoff(): OpenInSessionHandoff | undefined {
	return getContext<OpenInSessionHandoff | undefined>(KEY)
}

/** The mounted editor's hand-off for the item `route` names, or undefined when
 * no editor on screen publishes one for it (a legacy app, a detail page).
 * Matched on the item rather than taken as "the latest registered": a script
 * editor mounted in a flow's drawer registers too, and the rail wants the
 * page's own item. */
export function findMountedOpenInSessionSource(
	route: PreviewItemRoute
): OpenInSessionSource | undefined {
	const kind = route.kind === 'app' ? (route.raw_app ? 'raw_app' : undefined) : route.kind
	if (!kind) return undefined
	for (const handoff of mounted) {
		const source = handoff.source()
		const target = source?.target
		if (target && target.kind === kind && target.path === route.itemPath) return source
	}
	return undefined
}
