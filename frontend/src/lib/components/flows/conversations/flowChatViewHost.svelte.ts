import type { Chat, ChatAttachment, ChatMessage, ChatState } from 'windmill-chat'
import type {
	ChatSendRequestOptions,
	ChatViewHost
} from '$lib/components/copilot/chat/chatViewHost'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type { AIAutonomyMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
import { isPlanCardTool } from '$lib/components/copilot/chat/planMode'
import { AttachedFilesStore } from '$lib/components/copilot/chat/files/attachedFiles.svelte'
import { SessionArtifactsStore } from '$lib/components/copilot/chat/artifacts/artifactsState.svelte'
import type { AttachedBlob } from '$lib/components/copilot/chat/blobUtils'
import type { AttachedImage } from '$lib/components/copilot/chat/imageUtils'
import type { AttachedTextFile } from '$lib/components/copilot/chat/textFileUtils'
import { sendUserToast } from '$lib/toast'
import type { AttachmentsTarget } from './agentAttachmentInput'

export type FlowChatViewHostOptions = {
	/** The flow inputs sent next to `user_message` with every turn. */
	additionalInputs?: () => Record<string, any> | undefined
	/** The flow input the composer's attachments feed. Undefined where the flow has none:
	 * the chat then takes no attachments at all. */
	attachmentsTarget?: () => AttachmentsTarget | undefined
	/** Why attaching is off despite the flow taking attachments — no object storage, say.
	 * Undefined while the workspace has not answered: an explanation must not be a guess. */
	attachmentsUnavailable?: () => string | undefined
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

/** The chat's own signal for a send stopped before it ran; nothing to tell the reader. */
function isAbort(e: unknown): boolean {
	return e instanceof Error && e.name === 'AbortError'
}

type Queue = { text: string; images: AttachedImage[]; blobs: AttachedBlob[] }

function emptyQueue(): Queue {
	return { text: '', images: [], blobs: [] }
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

export function toDisplayMessages(messages: readonly ChatMessage[]): DisplayMessage[] {
	let userIndex = 0
	return messages.map((message, i): DisplayMessage => {
		switch (message.role) {
			case 'user':
				return {
					role: 'user',
					index: userIndex++,
					content: message.content,
					// Drives the shared Retry button.
					error: turnFailed(messages, i) || undefined
				}
			case 'tool': {
				const parameters = parseToolPayload(message.tool?.arguments)
				const result = parseToolPayload(message.tool?.result)
				const failed = message.success === false
				return {
					role: 'tool',
					tool_call_id: message.id,
					// The card's header is the row's text, which the server only words once the
					// tool has returned; until then the row says what is running.
					content: message.content || (message.tool ? `Running ${message.tool.name}` : ''),
					// Withheld for the copilot's two plan-mode names: `toolName` is what makes
					// ToolExecutionDisplay render a plan card, and an agent tool that happened to
					// share one would silently become one.
					toolName: isPlanCardTool(message.tool?.name) ? undefined : message.tool?.name,
					parameters,
					result,
					showDetails: parameters !== undefined || result !== undefined,
					error: failed ? message.content : undefined,
					isLoading: message.pending && message.tool?.status === 'running'
				}
			}
			default:
				return {
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
	 * turn's release would otherwise start a run from a panel that is gone. A send still
	 * uploading its attachments stops with the chat, which is the caller's to destroy; its
	 * draft is not handed back, since the composer it came from is gone too. */
	dispose() {
		this.#disposed = true
		this.#queue = emptyQueue()
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
			// A conversation reopened later comes back from the server under other message ids.
			this.#sentAttachments.clear()
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
		let images = options.images ?? []
		let blobs = options.blobs ?? []
		// The composer refuses an attachment-only send (requiresMessageText), so this is
		// the same rule at the other end: nothing runs without a message.
		if (!text) return false
		if (this.loading) {
			this.queueMessage(text, images, undefined, undefined, blobs)
			return true
		}
		if (this.#options.sendDisabled?.()) {
			// Refused, not dropped: the draft waits in the composer for sending to reopen.
			this.#aiChatInput?.prependText(text, images, [], blobs)
			return false
		}
		const target = this.#options.attachmentsTarget?.()
		// The inputs modal does not ask for this input, so a required one is enforced here.
		if (target?.required && images.length === 0 && blobs.length === 0) {
			sendUserToast('This chat needs a file with each message. Attach one to send.', true)
			this.#aiChatInput?.prependText(text, images, [], blobs)
			return false
		}
		const inputs = { ...(this.#options.additionalInputs?.() ?? {}) }
		// The attachments are this input's only editor: a value stored for it in the inputs
		// modal would otherwise ride along on every message.
		if (target) delete inputs[target.name]
		// The per-turn cap again, at the place the truncation would happen: the composer
		// enforces it as files are attached, but a queue built over several turns arrives
		// here as one send, and a scalar input keeps the first upload — uploading the rest
		// would strand them in storage while the transcript claimed they went.
		const cap = this.maxMessageAttachments
		if (cap !== undefined && images.length + blobs.length > cap) {
			const dropped = images.length + blobs.length - cap
			images = images.slice(0, cap)
			blobs = blobs.slice(0, Math.max(0, cap - images.length))
			sendUserToast(
				cap === 1
					? `This chat sends one attachment per message; ${dropped} file(s) were not sent.`
					: `This chat sends up to ${cap} attachments per message; ${dropped} file(s) were not sent.`,
				true
			)
		}
		const attachments: ChatAttachment[] = target
			? [...images, ...blobs].map((attachment, index) => ({
					name: attachment.name ?? `attachment-${index + 1}`,
					data: attachment.dataUrl,
					mediaType: attachment.mediaType
				}))
			: []
		this.#automaticScroll = true
		// A run that fails is reported through the chat's `onError` and as a failed message;
		// the promise itself only rejects when the chat refuses the turn outright — a turn
		// already running, an upload that failed, Stop pressed while it ran — and the draft
		// is then handed back rather than dropped. The composer took it before calling, so
		// nothing else would.
		const sending = this.#chat.sendMessage(text, {
			inputs: this.#options.additionalInputs?.() ? inputs : undefined,
			attachments,
			attachmentsInput: target
		})
		// `sendMessage` shows the user message before its first await, so the last one is this
		// turn's. Its id is stable across the server sync, which lets Retry resend the files.
		const last = this.#chat.getState().messages.at(-1)
		const sentId =
			last?.role === 'user' && last.pending && last.content === text ? last.id : undefined
		if (sentId && (images.length > 0 || blobs.length > 0)) {
			this.#sentAttachments.set(sentId, { images, blobs })
		}
		const turn = sending.catch((e) => {
			if (sentId) this.#sentAttachments.delete(sentId)
			if (this.#disposed) return
			if (attachments.length > 0 && !isAbort(e)) {
				sendUserToast(
					`Could not upload the attachments: ${e instanceof Error ? e.message : String(e)}`,
					true
				)
			}
			// What was queued behind it comes back too, after it: the chat publishes `idle`
			// when it withdraws the turn, and a queue left in place would be flushed as if
			// the turn had run.
			this.dequeueMessage()
			this.#aiChatInput?.prependText(text, images, [], blobs)
		})
		this.#turnDone = turn
		await turn
		// The files are kept only for a turn that failed, the one Retry is offered on: a base64
		// payload per sent file would otherwise pile up for as long as the panel lives.
		if (sentId) {
			const index = this.#state.messages.findIndex((m) => m.id === sentId)
			if (index === -1 || !turnFailed(this.#state.messages, index)) {
				this.#sentAttachments.delete(sentId)
			}
		}
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

	// One message typed while the turn runs, sent whole with its attachments once the turn
	// settles. Enter again appends a line rather than replacing what waits.
	#queue = $state<Queue>(emptyQueue())
	get queuedMessage(): string {
		return this.#queue.text
	}
	queuedContext = undefined
	get queuedImages(): AttachedImage[] {
		return this.#queue.images
	}
	queuedFiles: AttachedTextFile[] = []
	get queuedBlobs(): AttachedBlob[] {
		return this.#queue.blobs
	}
	queueMessage = (
		text: string,
		images: AttachedImage[] = [],
		_context?: unknown,
		_files?: unknown,
		blobs: AttachedBlob[] = []
	) => {
		const trimmed = text.trim()
		if (!trimmed && images.length === 0 && blobs.length === 0) return
		const queue = this.#queue
		this.#queue = {
			text: !trimmed ? queue.text : queue.text ? `${queue.text}\n${trimmed}` : trimmed,
			images: [...queue.images, ...images],
			blobs: [...queue.blobs, ...blobs]
		}
	}
	/** Put the queued draft back in the composer, attachments included. */
	dequeueMessage = () => {
		const { text, images, blobs } = this.#takeQueue()
		if (!text && images.length === 0 && blobs.length === 0) return
		this.#aiChatInput?.prependText(text, images, [], blobs)
	}
	flushQueuedMessage = () => {
		// Same rule as sendRequest, read before the queue is drained: a turn with no message
		// cannot run, and taking the queue for it would drop the attachments on the floor.
		if (!this.#queue.text || this.#disposed) return
		const { text, images, blobs } = this.#takeQueue()
		void this.sendRequest({ instructions: text, images, blobs })
	}
	#takeQueue(): Queue {
		const taken = this.#queue
		this.#queue = emptyQueue()
		return taken
	}
	setComposerStaged = () => {}
	clearComposerStaged = () => {}
	attachmentBytesExcluding = () => 0

	// Per-message actions
	storedImages = () => undefined
	/** Send the user message at this transcript position again. */
	/** The files each user message of this session went out with, for Retry. A message loaded
	 * from history has none recorded here and retries with its text alone. */
	#sentAttachments = new Map<string, { images: AttachedImage[]; blobs: AttachedBlob[] }>()
	retryRequest = (messageIndex: number) => {
		const message = this.#state.messages[messageIndex]
		if (!message || message.role !== 'user' || this.loading) return
		void this.sendRequest({
			instructions: message.content,
			...this.#sentAttachments.get(message.id)
		})
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
	// The input's shape alone: whether this chat takes attachments at all is a fact about the
	// flow, not about the workspace. Object storage decides whether it can right now, which is
	// `attachmentsUnavailableReason` — a state on the control rather than a reason to move the
	// input to the modal, where a file picker would be just as unable to upload.
	get supportsMessageAttachments(): boolean {
		return !!this.#options.attachmentsTarget?.()
	}
	get attachmentsUnavailableReason(): string | undefined {
		return this.#options.attachmentsUnavailable?.()
	}
	// An AI agent step refuses a run with no `user_message`.
	requiresMessageText = true
	// Attachments go to object storage for the worker to read, so a linked folder — a live
	// handle on the user's own disk — has no meaning here.
	supportsLinkedFolders = false
	attachmentsAsBlobs = true
	// A scalar flow input holds one file; sending more would upload every one and run with
	// the first, leaving the rest orphaned in storage and the transcript claiming otherwise.
	get maxMessageAttachments(): number | undefined {
		return this.#options.attachmentsTarget?.()?.multiple === false ? 1 : undefined
	}
	// What a provider actually takes. Anthropic's document block accepts base64
	// `application/pdf` and nothing else, so the wider set `is_document_mime`
	// (windmill-ai/src/ai_types.rs) claims — csv, html, plain, docx, xlsx — is rejected with a
	// 400 rather than read. Widen this only alongside a worker that inlines text as text.
	attachmentAccept = 'image/*,application/pdf,.pdf'
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
