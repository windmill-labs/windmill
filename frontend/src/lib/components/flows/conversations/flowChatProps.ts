/** What separates the chat from what sits above it. `boxed` is its own panel, corners
 *  clipped so the sidebar's edge follows them; `top` a dividing line under an enclosing
 *  header; `none` for a surface where the chat is the whole pane. */
export type ChatFrame = 'boxed' | 'top' | 'none'

export const FRAME_CLASS: Record<ChatFrame, string> = {
	boxed: 'border rounded-md',
	top: 'border-t',
	none: ''
}

/**
 * Which flow a chat is for, as something that holds still: `identity` where a surface has
 * one, since `path` follows the path field as its author types. `||`, not `??`: an empty
 * identity is no identity.
 */
export function chatFlowKey(props: { path: string; identity?: string }): string {
	return props.identity || props.path
}

const INPUTS_STORAGE_PREFIX = 'windmill_flow_chat_inputs_'

/**
 * The flow inputs the reader chose — the model among them. They belong to the flow and are
 * shared by every conversation in it, so they are held once per chat rather than once per
 * panel: two panels reading storage on their own would each keep the value it read.
 */
export function loadFlowChatInputs(props: {
	path: string
	identity?: string
}): Record<string, any> {
	try {
		const stored = localStorage.getItem(`${INPUTS_STORAGE_PREFIX}${chatFlowKey(props)}`)
		return stored ? JSON.parse(stored) : {}
	} catch (e) {
		console.error('Failed to load inputs from localStorage:', e)
		return {}
	}
}

export function saveFlowChatInputs(
	props: { path: string; identity?: string },
	values: Record<string, any>
): void {
	try {
		localStorage.setItem(`${INPUTS_STORAGE_PREFIX}${chatFlowKey(props)}`, JSON.stringify(values))
	} catch (e) {
		console.error('Failed to save inputs to localStorage:', e)
	}
}
