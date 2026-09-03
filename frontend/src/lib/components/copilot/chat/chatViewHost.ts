import { getContext, setContext } from 'svelte'
import type { AIMode, AIAutonomyMode } from './AIChatManager.svelte'
import { getAiChatManager } from './aiChatManagerContext'
import type { DisplayMessage, Tool } from './shared'
import type { ContextElement } from './context'
import type { AttachedBlob } from './blobUtils'
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
	blobs?: AttachedBlob[]
	/** Selected-context snapshot for this turn, in place of the live selection. Set
	 * whenever a send settles its context ahead of the turn. A host with no context
	 * of its own ignores it. */
	contextOverride?: ContextElement[]
	/** Where `contextOverride` came from. 'pinned': chips picked for THIS message, so
	 * they are consumed from the live selection on send. 'replay': an edit or retry
	 * resending an older message's context, already consumed long ago. */
	contextOverrideOrigin?: 'pinned' | 'replay'
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
	readonly queuedBlobs: AttachedBlob[]
	queueMessage: (
		text: string,
		images?: AttachedImage[],
		context?: ContextElement[],
		files?: AttachedTextFile[],
		blobs?: AttachedBlob[]
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
	/** The `+` menu's file entry and drag-and-drop onto the panel. Attachments ride
	 * one message; where they go afterwards is the host's business (see sendRequest). */
	supportsMessageAttachments: boolean
	/** The `+` menu's folder entries, backed by `attachedFiles`. A linked folder is a
	 * live handle on the user's disk, so only a host reading files in the browser has one. */
	supportsLinkedFolders: boolean
	/** `accept` for the file picker, and the drop filter. A host whose consumer only
	 * understands some formats narrows it so the rest are refused rather than ignored. */
	attachmentAccept: string
	/** Take non-image attachments verbatim (`blobs`) instead of decoding them to text.
	 * True where the bytes are forwarded somewhere — object storage — rather than read
	 * in the browser. */
	attachmentsAsBlobs: boolean
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
