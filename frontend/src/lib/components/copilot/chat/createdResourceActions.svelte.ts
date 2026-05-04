import { sendUserToast } from '$lib/toast'
import type { ToolDisplayAction } from './shared'

type MaybePromise<T> = T | Promise<T>
type ToolDisplayActionHandler = (action: ToolDisplayAction) => MaybePromise<void>

const toolDisplayActionHandlers = $state<Record<string, ToolDisplayActionHandler | undefined>>({})

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
	toolDisplayActionHandlers[type] = handler
	return () => {
		if (toolDisplayActionHandlers[type] === handler) {
			delete toolDisplayActionHandlers[type]
		}
	}
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
