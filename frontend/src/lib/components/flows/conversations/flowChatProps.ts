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
 * Which flow a chat is for, as something that holds still.
 *
 * `identity` where a surface has one, because `path` follows the path field as its author
 * types and anything filed under the flow has to survive that. A surface with no identity
 * to give has no such field either, so its path is already the stable answer.
 */
export function chatFlowKey(props: { path: string; identity?: string }): string {
	// `||`, not `??`: the editor builds `identity` from values that are both empty on a
	// surface with no flow of its own to name, and an empty identity is no identity.
	return props.identity || props.path
}

/**
 * What makes one chat a different chat: the flow it is for, in the workspace it runs in.
 * `FlowChat` builds its chat on this, so a rename does not replace it.
 */
export function chatIdentity(
	workspace: string | undefined,
	props: { path: string; identity?: string }
): string {
	return `${workspace ?? ''}:${chatFlowKey(props)}`
}
