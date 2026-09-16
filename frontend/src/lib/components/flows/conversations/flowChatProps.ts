import type { FlowModule } from '$lib/gen'
import type { ConversationKind } from './FlowChatManager.svelte'

/** What separates the chat from what sits above it. `boxed` is its own panel, corners
 *  clipped so the sidebar's edge follows them; `top` a dividing line under an enclosing
 *  header; `none` for a surface where the chat is the whole pane. */
export type ChatFrame = 'boxed' | 'top' | 'none'

/**
 * Shared by `FlowChat` and the `FlowChatPanel` it keys, which passes them straight through:
 * one declaration, so a prop cannot reach the panel without the wrapper accepting it.
 */
export interface FlowChatProps {
	onRunFlow: (
		userMessage: string,
		conversationId: string,
		additionalInputs?: Record<string, any>
	) => Promise<string | undefined>
	useStreaming?: boolean
	deploymentInProgress?: boolean
	/** The flow the chat runs and lists conversations for. Must be the path a run records,
	 *  or a conversation is stored under one path and looked for under another. */
	path: string
	/**
	 * What makes this a different chat, when that is not the path. An unsaved flow's path
	 * changes as its author types, and the chat is replaced whenever this changes — so the
	 * editor passes something that holds still for the flow it is editing.
	 */
	identity?: string
	/** The flow's own description, shown where the chat has room for it: the empty
	 * transcript, and the sidebar once a conversation has replaced it. */
	description?: string
	inputSchema?: Record<string, any>
	/** The flow's modules, used to find which inputs an AI agent step reads directly. */
	flowModules?: FlowModule[]
	/** Wider centered column, for the full-page chat. */
	wideLayout?: boolean
	frame?: ChatFrame
	/** Which chats the sidebar lists before the reader filters it themselves. */
	conversationKind?: ConversationKind
	/** Whether a turn may run while another chat's is still going. Off where one run
	 * owns the surface — the editor's panel shows it on the graph. */
	parallelTurns?: boolean
}

/**
 * Which flow a chat is for, as something that holds still.
 *
 * `identity` where a surface has one, because `path` follows the path field as its author
 * types and anything filed under the flow has to survive that — the panel is deliberately
 * not remounted for a path change. A surface with no identity to give has no such field
 * either, so its path is already the stable answer.
 */
export function chatFlowKey(props: Pick<FlowChatProps, 'path' | 'identity'>): string {
	// `||`, not `??`: the editor builds `identity` from values that are both empty on a
	// surface with no flow of its own to name, and an empty identity is no identity.
	return props.identity || props.path
}

/**
 * What makes one chat a different chat: the flow it is for, in the workspace it runs in.
 * `FlowChat` keys the panel on this, so nothing inside a mounted panel can see it change.
 */
export function chatIdentity(
	workspace: string | undefined,
	props: Pick<FlowChatProps, 'path' | 'identity'>
): string {
	return `${workspace ?? ''}:${chatFlowKey(props)}`
}
