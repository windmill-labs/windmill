import type { ContextElement } from '$lib/components/copilot/chat/context'
import type { BulkKind } from '$lib/components/home/homeSelection.svelte'

/** Resolves once the element is in the draft, so several attach in the order given. */
type Attach = (element: ContextElement) => Promise<void>

/** What the composer can take as context from a workspace item: a script, a flow or a code
 * app, as "+" → Mention file builds it. A visual-builder app or an agent has no context
 * element, so it gets none. */
export function itemContextElement(item: {
	kind: BulkKind
	path: string
	summary?: string
	rawApp?: boolean
}): ContextElement | undefined {
	const type =
		item.kind === 'script'
			? 'workspace_script'
			: item.kind === 'flow'
				? 'workspace_flow'
				: item.kind === 'app' && item.rawApp
					? 'workspace_app'
					: undefined
	if (!type) return undefined
	return { type, path: item.path, title: item.path, summary: item.summary, deletable: true }
}

let attach = $state<Attach | undefined>(undefined)

/**
 * The way from anywhere on Home into Home's composer: a row's "Ask AI" hands its item over
 * here, and the composer takes it as context. Only a mounted composer registers, so a page
 * whose composer is removed, hidden or not Home offers no button that would go nowhere.
 */
export const homeComposerContext = {
	get available(): boolean {
		return attach !== undefined
	},
	async attach(element: ContextElement): Promise<void> {
		await attach?.(element)
	},
	/** Returns the unregister, which only clears its own registration: a remount registers
	 * the new composer before the old one is destroyed. */
	register(fn: Attach): () => void {
		attach = fn
		return () => {
			if (attach === fn) attach = undefined
		}
	}
}
