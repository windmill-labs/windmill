import { sendUserToast } from '$lib/toast'
import type { ToolDisplayAction } from './shared'

type MaybePromise<T> = T | Promise<T>
type ToolDisplayActionHandler = (action: ToolDisplayAction) => MaybePromise<void>

const toolDisplayActionHandlers = $state<Record<string, ToolDisplayActionHandler | undefined>>({})
// Every registration per type, latest last: a page that takes over a type from the layout
// (the sessions page opens items in its panel rather than in drawers) hands it back on
// unmount instead of leaving the type unhandled.
const registrations: Record<string, ToolDisplayActionHandler[]> = {}

function formatUnknownError(error: unknown): string {
	if (error instanceof Error) {
		return error.message
	}
	return String(error)
}

export function registerToolDisplayActionHandler(
	type: ToolDisplayAction['type'],
	handler: ToolDisplayActionHandler
): () => void {
	const stack = (registrations[type] ??= [])
	stack.push(handler)
	toolDisplayActionHandlers[type] = handler
	return () => {
		const at = stack.lastIndexOf(handler)
		if (at < 0) return
		stack.splice(at, 1)
		const current = stack[stack.length - 1]
		if (current) toolDisplayActionHandlers[type] = current
		else delete toolDisplayActionHandlers[type]
	}
}

/**
 * Reactive: reads the `$state` registry, so a component re-renders when a page mounts or
 * unmounts its handler. Offering an action without checking this yields an affordance whose
 * only outcome is the unavailable-action toast.
 */
export function hasToolDisplayActionHandler(type: ToolDisplayAction['type']): boolean {
	return toolDisplayActionHandlers[type] !== undefined
}

export async function runToolDisplayAction(action: ToolDisplayAction): Promise<void> {
	const handler = toolDisplayActionHandlers[action.type]
	if (!handler) {
		sendUserToast('This action is not available right now.', true)
		return
	}

	try {
		await handler(action)
	} catch (error) {
		sendUserToast(`Could not run action "${action.label}": ${formatUnknownError(error)}`, true)
	}
}
