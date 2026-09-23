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
