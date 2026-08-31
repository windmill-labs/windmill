import { getContext, setContext } from 'svelte'
import type { AIMode, AIAutonomyMode } from './AIChatManager.svelte'
import { getAiChatManager } from './aiChatManagerContext'
import type { DisplayMessage, Tool } from './shared'
import type { ContextElement } from './context'
import type { AttachedImage } from './imageUtils'
import type { AttachedTextFile } from './textFileUtils'
import type { PasteAttachment } from './pasteTokens'
import type { AttachedFilesStore } from './files/attachedFiles.svelte'
import type { SessionArtifactsStore } from './artifacts/artifactsState.svelte'
import type { ArtifactVersionTarget } from '$lib/components/sessions/previewRouter'
import type { FlowAIChatHelpers } from './flow/core'
import type { AppAIChatHelpers } from './app/core'
import type AIChatInput from './AIChatInput.svelte'

export type ChatSendRequestOptions = {
	instructions?: string
	pastes?: PasteAttachment[]
	images?: AttachedImage[]
	files?: AttachedTextFile[]
}

/**
 * What the chat view components (AIChatDisplay and everything it renders) need
 * from whatever is driving the conversation. AIChatManager implements it for the
 * copilot's own LLM loop; FlowChatViewHost implements it over a flow run's
 * conversation so both chats render through the same components.
 *
 * `mode` is what gates the copilot-only chrome — context picker, file
 * attachments, MCP, autonomy, suggestions. A host that leaves it undefined gets
 * a bare transcript + composer, which is what a non-copilot host wants.
 */
export interface ChatViewHost {
	// Transcript
	displayMessages: DisplayMessage[]
	/** API-level messages. Only the count is read (context usage visibility). */
	messages: readonly unknown[]
	contextTokens: number
	loading: boolean
	loadingLabel: string | undefined
	compacting: boolean
	currentReply: string
	currentReasoning: string
	currentReasoningActive: boolean
	readonly reasoningHiddenIndicatorLabel: string | undefined
	readonly automaticScroll: boolean
	enableAutomaticScroll: () => void
	disableAutomaticScroll: () => void

	// Composer
	instructions: string
	readonly sendInFlight: boolean
	/** Resolves to whether the draft was consumed as a turn. */
	sendRequest: (options?: ChatSendRequestOptions) => Promise<boolean | undefined>
	cancel: (reason?: string) => void
	setAiChatInput: (aiChatInput: AIChatInput | null) => void
	queuedMessage: string
	queuedContext: ContextElement[] | undefined
	readonly queuedImages: AttachedImage[]
	readonly queuedFiles: AttachedTextFile[]
	queueMessage: (
		text: string,
		images?: AttachedImage[],
		context?: ContextElement[],
		files?: AttachedTextFile[]
	) => void
	dequeueMessage: () => void
	setComposerStaged: (key: string, editingIndex: number | null, bytes: number) => void
	clearComposerStaged: (key: string) => void
	attachmentBytesExcluding: (selfKey: string) => number

	// Per-message actions
	storedImages: (displayMessageIndex: number) => AttachedImage[] | undefined
	retryRequest: (messageIndex: number) => void
	restartGeneration: (
		displayMessageIndex: number,
		newContent?: string,
		pastes?: PasteAttachment[],
		images?: AttachedImage[],
		editedContext?: ContextElement[],
		files?: AttachedTextFile[]
	) => void | Promise<void>
	handleUserQuestionAnswer: (toolId: string, choices: string[]) => boolean
	handleToolConfirmation: (toolId: string, confirmed: boolean) => void

	// Copilot-only surfaces. Left undefined/false by hosts that have no LLM loop
	// of their own; the chrome they drive hides itself.
	mode?: AIMode
	isSessionChat: boolean
	/** Model + reasoning picker. Off where the model is configured elsewhere. */
	supportsModelSettings: boolean
	/** Click a user message to edit and resend it. Needs a host that can rewind
	 * its own transcript, which a host replaying a server-side run cannot. */
	supportsMessageEditing: boolean
	tools: Tool<any>[]
	autonomyMode: AIAutonomyMode
	setAutonomyMode: (mode: AIAutonomyMode) => void
	readonly autoAcceptEditsActive: boolean
	readonly autoAcceptEditsAvailable: boolean
	readonly autoAcceptToolConfirmationsAvailable: boolean
	readonly planModeAvailable: boolean
	attachedFiles: AttachedFilesStore
	artifacts: SessionArtifactsStore
	openArtifact?: (artifactId: string, name: string, version?: ArtifactVersionTarget) => void
	flowAiChatHelpers?: FlowAIChatHelpers
	appAiChatHelpers?: AppAIChatHelpers
}

const CHAT_VIEW_HOST_CONTEXT_KEY = 'chatViewHost'

export function setChatViewHost(host: ChatViewHost) {
	setContext(CHAT_VIEW_HOST_CONTEXT_KEY, host)
}

/**
 * Resolve the host driving the chat in this subtree. Falls back to the
 * AIChatManager (scoped instance or app-wide singleton) so every existing
 * copilot chat keeps working without setting anything.
 */
export function getChatViewHost(): ChatViewHost {
	return getContext<ChatViewHost>(CHAT_VIEW_HOST_CONTEXT_KEY) ?? getAiChatManager()
}
