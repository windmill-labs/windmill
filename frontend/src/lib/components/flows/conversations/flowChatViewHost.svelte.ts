import type { Chat, ChatMessage, ChatState } from 'windmill-chat'
import type {
	ChatSendRequestOptions,
	ChatViewHost
} from '$lib/components/copilot/chat/chatViewHost'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type { AIAutonomyMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
import { isPlanCardTool } from '$lib/components/copilot/chat/planMode'
import { AttachedFilesStore } from '$lib/components/copilot/chat/files/attachedFiles.svelte'
import { SessionArtifactsStore } from '$lib/components/copilot/chat/artifacts/artifactsState.svelte'
import type { AttachedImage } from '$lib/components/copilot/chat/imageUtils'
import type { AttachedTextFile } from '$lib/components/copilot/chat/textFileUtils'

export type FlowChatViewHostOptions = {
	/** The flow inputs sent next to `user_message` with every turn. */
	additionalInputs?: () => Record<string, any> | undefined
	/** The workspace the transcript's paths resolve against. */
	workspace?: () => string | undefined
	/** Whether sending is refused right now (a deployment in progress, say). The composer
	 * is disabled on the same condition; this covers the sends the composer does not
	 * make itself: a queued message going out, a retry. */
	sendDisabled?: () => boolean
}

function isBusy(status: ChatState['status']): boolean {
	return status === 'submitted' || status === 'streaming'
}

/** A tool's arguments or result as the card shows them: parsed where the string is JSON. */
function parseToolPayload(raw: string | undefined): unknown {
	if (raw === undefined || raw === '') return undefined
	try {
		return JSON.parse(raw)
	} catch {
		return raw
	}
}

/**
 * Whether the turn the user message at `index` started failed: its last row before the
 * next user message reports `success: false`. The last row, not any row: a tool call can
 * fail and the agent still answer, and that turn completed.
 */
export function turnFailed(messages: readonly ChatMessage[], index: number): boolean {
	let last: ChatMessage | undefined
	for (let i = index + 1; i < messages.length; i++) {
		const message = messages[i]
		if (message.role === 'user') break
		last = message
	}
	return last?.success === false
}

/** Whether the latest turn failed, per `turnFailed`. False before any turn. */
function lastTurnFailed(messages: readonly ChatMessage[]): boolean {
	for (let i = messages.length - 1; i >= 0; i--) {
		if (messages[i].role === 'user') return turnFailed(messages, i)
	}
	return false
}

/** The name the worker gives the structured-output tool: suffixed when an agent tool already has it. */
const STRUCTURED_OUTPUT_CALL = /^structured_output(_\d+)?$/

export function toDisplayMessages(messages: readonly ChatMessage[]): DisplayMessage[] {
	let userIndex = 0
	return messages.flatMap((message, i): DisplayMessage[] => {
		switch (message.role) {
			case 'user':
				return [
					{
						role: 'user',
						index: userIndex++,
						content: message.content,
						// Drives the shared Retry button.
						error: turnFailed(messages, i) || undefined
					}
				]
			case 'tool': {
				const parameters = parseToolPayload(message.tool?.arguments)
				const result = parseToolPayload(message.tool?.result)
				const failed = message.success === false
				// A call the turn finished without, stopped or lost before its row was written. The
				// call a structured answer streams as never gets a result of its own, the answer
				// being the turn's text, so it is not one.
				const unfinished =
					message.tool &&
					!message.pending &&
					!message.content &&
					!STRUCTURED_OUTPUT_CALL.test(message.tool.name)
						? `${message.tool.name} did not finish`
						: undefined
				const call: DisplayMessage = {
					role: 'tool',
					tool_call_id: message.id,
					// The card's header is the row's text, which the server only words once the
					// tool has returned; until then the row says what is running.
					content:
						message.content || unfinished || (message.tool ? `Running ${message.tool.name}` : ''),
					// Withheld for the copilot's two plan-mode names: `toolName` is what makes
					// ToolExecutionDisplay render a plan card, and an agent tool that happened to
					// share one would silently become one.
					toolName: isPlanCardTool(message.tool?.name) ? undefined : message.tool?.name,
					parameters,
					result,
					showDetails: parameters !== undefined || result !== undefined,
					error: failed ? message.content : unfinished,
					isLoading: message.pending && message.tool?.status === 'running'
				}
				// The tool card has no thinking section: the thinking that led to the call reads
				// as a card of its own, just before it.
				return message.reasoning
					? [
							{
								role: 'assistant',
								content: '',
								reasoning: message.reasoning,
								stepName: message.stepName,
								jobId: message.jobId,
								createdAt: message.createdAt
							},
							call
						]
					: [call]
			}
			default:
				return [
					{
						role: 'assistant',
						content: message.content,
						// Only the message a turn is still writing: a finalized reasoning-only
						// message must not look in progress.
						streaming: message.pending || undefined,
						reasoning: message.reasoning,
						stepName: message.stepName,
						jobId: message.jobId,
						createdAt: message.createdAt
					}
				]
		}
	})
}

/**
 * Renders a flow run's conversation, as the `windmill-chat` SDK keeps it, through the
 * copilot's chat components. The turn is a flow job rather than an LLM call this host
 * makes, so what it can offer is what the SDK's `Chat` can: a message in, an answer
 * streamed back, Stop. Every copilot-only field is answered with "no" (see ChatViewHost).
 *
 * The host owns its subscription to the chat, so a panel swapping chats mounts a new one.
 */
export class FlowChatViewHost implements ChatViewHost {
	#chat: Chat
	#options: FlowChatViewHostOptions
	#state = $state.raw<ChatState>() as ChatState
	#unsubscribe: () => void

	constructor(chat: Chat, options: FlowChatViewHostOptions = {}) {
		this.#chat = chat
		this.#options = options
		this.#state = chat.getState()
		this.#unsubscribe = chat.subscribe((state) => this.#onState(state))
	}

	#disposed = false
	/** Stops following the chat, and drops what was queued: a flush still waiting on the
	 * turn's release would otherwise start a run from a panel that is gone. The chat itself
	 * is the caller's to destroy. */
	dispose() {
		this.#disposed = true
		this.#queued = ''
		this.#unsubscribe()
	}

	/** The latest `ChatState`, for what the interface reads beyond the seam (paging, loading). */
	get state(): ChatState {
		return this.#state
	}

	#onState(state: ChatState) {
		const previous = this.#state
		this.#state = state
		if (previous.conversationId !== state.conversationId) {
			// A conversation opens at its end, whatever the reader was doing in the last one.
			this.#automaticScroll = true
			// The queue was typed into the conversation that just went away; a message sent
			// after the switch would ride out of the wrong one, so it goes back to the composer.
			this.dequeueMessage()
			return
		}
		if (isBusy(previous.status) && !isBusy(state.status)) {
			// The turn settled. What was typed during it goes out once the turn is released,
			// not now: the chat publishes `idle` from inside its own `sendMessage`, which still
			// counts the turn as open until it returns, and a send made before that would be
			// refused as a second turn. After a failure it goes back to the composer instead,
			// where the reader would rather look at the error than pile on. A failed flow
			// settles as `idle` too, with its error as the answer, so the messages decide.
			const succeeded = state.status === 'idle' && !lastTurnFailed(state.messages)
			if (succeeded) void this.#turnDone.then(this.flushQueuedMessage)
			else this.dequeueMessage()
		}
	}

	// Transcript
	displayMessages = $derived.by(() => toDisplayMessages(this.#state.messages))
	get messages(): readonly unknown[] {
		return this.#state.messages
	}
	contextTokens = 0
	get operatingWorkspace(): string | undefined {
		return this.#options.workspace?.()
	}
	get loading(): boolean {
		return isBusy(this.#state.status)
	}
	runHeldElsewhere = false
	loadingLabel = undefined
	compacting = false
	// The answer streams into the message list itself, so the live lanes stay empty.
	currentReply = ''
	currentReasoning = ''
	currentReasoningActive = false
	reasoningHiddenIndicatorLabel = undefined
	#automaticScroll = $state(true)
	get automaticScroll(): boolean {
		return this.#automaticScroll
	}
	enableAutomaticScroll = () => {
		this.#automaticScroll = true
	}
	disableAutomaticScroll = () => {
		this.#automaticScroll = false
	}

	// Composer
	instructions = ''
	// The user message lands in the transcript before `sendMessage` awaits anything.
	sendInFlight = false
	sendRequest = async (options: ChatSendRequestOptions = {}): Promise<boolean> => {
		const text = options.instructions?.trim() ?? ''
		if (!text) return false
		if (this.loading) {
			this.queueMessage(text)
			return true
		}
		if (this.#options.sendDisabled?.()) {
			// Refused, not dropped: the text waits in the composer for sending to reopen.
			this.#aiChatInput?.prependText(text)
			return false
		}
		this.#automaticScroll = true
		// A run that fails is reported through the chat's `onError` and as a failed message;
		// the promise itself only rejects when the chat refuses the turn outright, and the
		// text is then handed back rather than dropped.
		const turn = this.#chat
			.sendMessage(text, { inputs: this.#options.additionalInputs?.() })
			.catch(() => this.#aiChatInput?.prependText(text))
		this.#turnDone = turn
		await turn
		return true
	}
	/** Settles when the chat has released the last turn this host started. */
	#turnDone: Promise<unknown> = Promise.resolve()
	cancel = () => {
		// Stop means stop: what was typed during the run goes back to the composer rather
		// than waiting there to go out after some later turn settles.
		this.dequeueMessage()
		void this.#chat.stop()
	}
	// Typed off the interface: a Svelte component's own type resolves differently
	// across import specifiers, and the two would then not be assignable.
	#aiChatInput: Parameters<ChatViewHost['setAiChatInput']>[0] = null
	setAiChatInput: ChatViewHost['setAiChatInput'] = (aiChatInput) => {
		this.#aiChatInput = aiChatInput
	}

	// One message typed while the turn runs, sent whole once it settles. Enter again
	// appends a line rather than replacing what waits.
	#queued = $state('')
	get queuedMessage(): string {
		return this.#queued
	}
	queuedContext = undefined
	queuedImages: AttachedImage[] = []
	queuedFiles: AttachedTextFile[] = []
	queueMessage = (text: string) => {
		const trimmed = text.trim()
		if (!trimmed) return
		this.#queued = this.#queued ? `${this.#queued}\n${trimmed}` : trimmed
	}
	/** Put the queued draft back in the composer. */
	dequeueMessage = () => {
		const text = this.#queued
		if (!text) return
		this.#queued = ''
		this.#aiChatInput?.prependText(text)
	}
	flushQueuedMessage = () => {
		const text = this.#queued
		if (!text || this.#disposed) return
		this.#queued = ''
		void this.sendRequest({ instructions: text })
	}
	setComposerStaged = () => {}
	clearComposerStaged = () => {}
	attachmentBytesExcluding = () => 0

	// Per-message actions
	storedImages = () => undefined
	/** Send the user message at this transcript position again. The position is in
	 * `displayMessages`, which holds more entries than the chat's messages. */
	retryRequest = (messageIndex: number) => {
		const message = this.displayMessages[messageIndex]
		if (!message || message.role !== 'user' || this.loading) return
		void this.sendRequest({ instructions: message.content })
	}
	restartGeneration = () => {}
	handleUserQuestionAnswer = () => false
	handleToolConfirmation = () => {}
	hasPendingRunForm = false
	isRunFormPending = () => false

	// Copilot-only surfaces
	mode = undefined
	isSessionChat = false
	supportsModelSettings = false
	supportsMessageEditing = false
	supportsMessageAttachments = false
	// An AI agent step refuses a run with no `user_message`.
	requiresMessageText = true
	supportsLinkedFolders = false
	attachmentAccept = ''
	tools = []
	// The enum's value, written out so this module never imports the copilot manager at
	// runtime: its unit test would otherwise load the manager and the editor it pulls in.
	autonomyMode = 'default' as AIAutonomyMode.DEFAULT
	setAutonomyMode = () => {}
	autoAcceptEditsActive = false
	autoAcceptEditsAvailable = false
	autoAcceptToolConfirmationsAvailable = false
	planModeAvailable = false
	attachedFiles = new AttachedFilesStore()
	artifacts = new SessionArtifactsStore()
}
