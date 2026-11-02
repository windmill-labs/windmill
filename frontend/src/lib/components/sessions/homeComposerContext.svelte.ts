import type { ContextElement } from '$lib/components/copilot/chat/context'
import type { BulkKind } from '$lib/components/home/homeSelection.svelte'
import type { SessionRuntime } from './sessionRuntime.svelte'

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

type Registration = {
	/** The draft the composer shows. */
	sessionId: string
	attach: Attach
	focus: () => void
	/** The box's accent outline, for a composer the user is sent to without it moving. */
	flash: () => void
	/** Assistant settings on `section`, focused on `root`'s folder. */
	openSettings: (section: 'instructions' | 'skills' | 'mcp', root: string) => void
	runtime: SessionRuntime
}

// Raw: the runtime is shared with the session page and must stay the same object there.
let registered = $state.raw<Registration | undefined>(undefined)

/**
 * The way from anywhere on Home into Home's composer: a row's "Ask AI" hands its item over
 * here, and the composer takes it as context. Only a mounted composer registers, so a page
 * whose composer is removed, hidden or not Home offers no button that would go nowhere.
 */
export const homeComposerContext = {
	get available(): boolean {
		return registered !== undefined
	},
	/** The runtime of the draft in Home's composer, whose chat manager the tree's folder badges
	 * read. Read from here because Home must not import the runtime's code (it reaches monaco). */
	get runtime(): SessionRuntime | undefined {
		return registered?.runtime
	},
	async attach(element: ContextElement): Promise<void> {
		await registered?.attach(element)
	},
	/** Whether Home's composer is on screen with this draft. */
	holds(sessionId: string): boolean {
		return registered?.sessionId === sessionId
	},
	focus(): void {
		registered?.focus()
	},
	flash(): void {
		registered?.flash()
	},
	openSettings(section: 'instructions' | 'skills' | 'mcp', root: string): void {
		registered?.openSettings(section, root)
	},
	/** Returns the unregister, which only clears its own registration: a remount registers
	 * the new composer before the old one is destroyed. */
	register(registration: Registration): () => void {
		registered = registration
		return () => {
			if (registered === registration) registered = undefined
		}
	}
}
