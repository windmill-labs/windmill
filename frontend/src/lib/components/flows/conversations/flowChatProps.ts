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
	path: string
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
