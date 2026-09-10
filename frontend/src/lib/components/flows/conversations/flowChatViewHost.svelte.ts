import type {
	ChatSendRequestOptions,
	ChatViewHost
} from '$lib/components/copilot/chat/chatViewHost'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type { ChatMessage, FlowChatManager } from './FlowChatManager.svelte'
import { AIAutonomyMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
import { isPlanCardTool } from '$lib/components/copilot/chat/planMode'
import { ToolCallStore, type ToolCallDetails } from './toolCallContext.svelte'
import { AttachedFilesStore } from '$lib/components/copilot/chat/files/attachedFiles.svelte'
import { SessionArtifactsStore } from '$lib/components/copilot/chat/artifacts/artifactsState.svelte'
import { dataUrlToBlob, type AttachedBlob } from '$lib/components/copilot/chat/blobUtils'
import type { AttachedImage } from '$lib/components/copilot/chat/imageUtils'
import type { AttachedTextFile } from '$lib/components/copilot/chat/textFileUtils'
import { HelpersService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { randomUUID } from '$lib/utils/uuid'
import {
	attachmentsToMessageInputs,
	MessageInputsStore,
	type MessageInputs
} from './messageInputContext.svelte'

/** A row that needs no job behind it. */
const EMPTY_TOOL_CALL: ToolCallDetails = {}

/** What an AI agent step reads out of `user_attachments`. */
type S3Attachment = { s3: string; filename?: string }

/** The flow input the composer's attachments feed, and whether it holds a list. */
export type AttachmentsTarget = { name: string; multiple: boolean }

export type FlowChatViewHostOptions = {
	additionalInputs?: () => Record<string, any> | undefined
	/** Called once the turn is dispatched, to clear anything that rides one message. */
	onSent?: () => void
	attachmentsTarget?: () => AttachmentsTarget | undefined
	workspace?: () => string | undefined
	/** Off when the workspace has no object storage — there is nowhere to upload to. */
	canAttach?: () => boolean
	/** Flow inputs the composer renders a control of its own for, so a message does not
	 * repeat them as context chips. */
	inputsShownInComposer?: () => string[]
}

/**
 * A tool's arguments and result reach us as strings: the provider's JSON for the call, and
 * whatever the tool returned, which is often but not always JSON. Parsed where it parses so
 * the card can fold it, kept verbatim where it does not.
 */
function parseToolPayload(raw: string | null | undefined): any {
	if (raw === undefined || raw === null || raw === '') return undefined
	try {
		return JSON.parse(raw)
	} catch {
		return raw
	}
}

function toDisplayMessage(
	message: ChatMessage,
	userIndex: number,
	showStepNames: boolean,
	inputs: MessageInputsStore,
	toolCalls: ToolCallStore,
	failed: boolean,
	pendingInputs: MessageInputs | undefined
): DisplayMessage {
	switch (message.message_type) {
		case 'user': {
			// What the turn ran with, read back from its job — the message row itself
			// keeps only the text. Renders through the same lanes the copilot uses.
			// The row the composer just added has no job id yet, so until the run is
			// persisted its own attachments stand in.
			const { images, contextElements } = message.job_id
				? inputs.get(message.job_id)
				: (pendingInputs ?? { images: [], contextElements: [] })
			return {
				role: 'user',
				index: userIndex,
				content: message.content,
				// Drives the shared Retry button: the turn this message started failed.
				error: failed || undefined,
				images: images.length > 0 ? images : undefined,
				contextElements: contextElements.length > 0 ? contextElements : undefined
			}
		}
		case 'tool': {
			const failed = message.success === false
			// While the turn streams, the events carry the call; afterwards the same details
			// come from the tool's own job. A row shows whichever it has.
			const streamed: ToolCallDetails = {
				toolName: message.tool_name,
				parameters: parseToolPayload(message.tool_arguments),
				result: parseToolPayload(message.tool_result)
			}
			// A row that stored its own call — an MCP tool, or one still streaming — has
			// nothing to learn from a job, and opening a conversation asks for one per row.
			const carriesItsOwnCall = streamed.parameters !== undefined || streamed.result !== undefined
			const fromJob = carriesItsOwnCall ? EMPTY_TOOL_CALL : toolCalls.get(message.job_id)
			const toolName = streamed.toolName ?? fromJob.toolName
			const parameters = streamed.parameters ?? fromJob.parameters
			const result = failed ? undefined : (streamed.result ?? fromJob.result)
			return {
				role: 'tool',
				tool_call_id: message.id,
				content: message.content,
				// Withheld for the copilot's two plan-mode names: `toolName` is what makes
				// ToolExecutionDisplay render a plan card, and an agent tool that happened to
				// share one would silently become one.
				toolName: isPlanCardTool(toolName) ? undefined : toolName,
				parameters,
				result,
				// The card's fold is opt-in (ToolExecutionDisplay reads showDetails), so it is
				// offered only when there is a call or a result behind it to reveal.
				showDetails: parameters !== undefined || result !== undefined,
				error: failed ? message.content : undefined,
				isLoading: message.loading
			}
		}
		default:
			return {
				role: 'assistant',
				content: message.content,
				streaming: message.streaming,
				reasoning: message.reasoning ?? undefined,
				stepName: showStepNames ? (message.step_name ?? undefined) : undefined,
				// The run behind the answer, so a reader can open what produced it. Absent on
				// the temp message a stream builds, which has no job id until it settles.
				jobId: message.job_id || undefined,
				createdAt: message.created_at
			}
	}
}

/**
 * Whether the turn a user message started came back unsuccessful. The answer is on
 * the messages that follow it, up to the next user message: an AI agent step or the
 * flow itself writes one with `success` false.
 */
function turnFailed(messages: ChatMessage[], userIndex: number): boolean {
	for (let i = userIndex + 1; i < messages.length; i++) {
		if (messages[i].message_type === 'user') return false
		if (messages[i].success === false) return true
	}
	return false
}

/**
 * Renders a flow run's conversation through the AI session chat components. The
 * turn itself is a flow job, so everything the copilot's own loop owns — context
 * elements, attachments, autonomy, model choice — is absent here, and the chrome
 * driving it hides itself (see ChatViewHost).
 */
export class FlowChatViewHost implements ChatViewHost {
	#manager: FlowChatManager
	#options: FlowChatViewHostOptions

	constructor(manager: FlowChatManager, options: FlowChatViewHostOptions = {}) {
		this.#manager = manager
		this.#options = options
	}

	// The step name says which AI agent step wrote a message, so it only tells the
	// reader anything once a conversation holds more than one. Counted over the
	// transcript rather than over the flow's current steps: a conversation outlives
	// edits to the flow, so it can carry labels from a shape the flow no longer has.
	#showStepNames = $derived.by(
		() => new Set(this.#manager.messages.map((m) => m.step_name).filter(Boolean)).size > 1
	)

	// What a turn was sent with, by the id of the row the composer added for it. A row
	// added here never gains a job id — the poller drops user rows from its response — so
	// these stay the only record of that turn's attachments until the conversation is
	// reloaded from the server and every row comes back with its run.
	#sentInputs = $state<Record<string, MessageInputs>>({})

	#messageInputs = new MessageInputsStore(
		() => this.#options.workspace?.(),
		() => new Set(this.#options.inputsShownInComposer?.() ?? [])
	)
	#toolCalls = new ToolCallStore(() => this.#options.workspace?.())

	displayMessages = $derived.by(() => {
		let userIndex = 0
		const showStepNames = this.#showStepNames
		const messages = this.#manager.messages
		return messages.map((message, i) =>
			toDisplayMessage(
				message,
				message.message_type === 'user' ? userIndex++ : -1,
				showStepNames,
				this.#messageInputs,
				this.#toolCalls,
				message.message_type === 'user' && turnFailed(messages, i),
				this.#sentInputs[message.id]
			)
		)
	})

	/**
	 * Send the message at this transcript position again. Its text only: the turn runs with
	 * the composer's current inputs and no attachments, since a failed turn's uploads are
	 * not held anywhere the composer can reach.
	 */
	retryRequest = (messageIndex: number) => {
		const message = this.#manager.messages[messageIndex]
		if (!message || message.message_type !== 'user' || this.loading) return
		void this.sendRequest({ instructions: message.content })
	}
	messages: readonly unknown[] = []
	contextTokens = 0
	operatingWorkspace = $derived.by(() => this.#options.workspace?.())
	loading = $derived.by(
		() => this.#manager.isLoading || this.#manager.isWaitingForResponse || this.#uploading
	)
	// A flow run is followed from its job, so another tab holds nothing this one can't read.
	runHeldElsewhere = false
	loadingLabel = undefined
	compacting = false
	currentReply = ''
	// The turn's thinking while it streams; it moves onto the answer once that starts.
	currentReasoning = $derived.by(() => this.#manager.currentReasoning)
	currentReasoningActive = $derived.by(() => this.#manager.isReasoningActive)
	reasoningHiddenIndicatorLabel = undefined

	#automaticScroll = $state(true)
	get automaticScroll() {
		return this.#automaticScroll
	}
	enableAutomaticScroll = () => {
		this.#automaticScroll = true
	}
	disableAutomaticScroll = () => {
		this.#automaticScroll = false
	}

	instructions = ''
	// The flow run is the send: it is in flight for as long as the job is.
	get sendInFlight() {
		return this.#manager.isLoading
	}
	// True while attachments are uploading — before the flow job exists, so
	// `manager.isLoading` cannot cover it and the composer would look idle.
	#uploading = $state(false)

	/** Put a refused turn back in the composer, which took the draft before calling. */
	#restoreToComposer(options: ChatSendRequestOptions) {
		this.#aiChatInput?.prependText(
			options.instructions ?? '',
			options.images ?? [],
			[],
			options.blobs ?? []
		)
	}

	sendRequest = async (options: ChatSendRequestOptions = {}) => {
		const text = options.instructions?.trim() ?? ''
		const args = { ...(this.#options.additionalInputs?.() ?? {}) }
		const target = this.#options.attachmentsTarget?.()
		let images = options.images ?? []
		let blobs = options.blobs ?? []
		// The composer refuses an attachment-only send (requiresMessageText), so this is
		// the same rule at the other end: nothing runs without a message.
		if (!text) return false
		// The per-turn cap again, at the place the truncation would happen: the composer
		// enforces it as files are attached, but a queue built over several turns arrives
		// here as one send, and a scalar input keeps `uploaded[0]` — uploading the rest
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
		const attachments = [...images, ...blobs]
		// Read before the upload below: the reader can pick another chat while it runs, and
		// the turn belongs to the one they sent it from.
		const conversationId = this.#manager.selectedConversationId
		let sentInputs: MessageInputs | undefined
		if (target && attachments.length > 0) {
			this.#uploading = true
			this.#manager.isDispatchingTurn = true
			try {
				const uploaded = await this.#uploadAttachments(attachments)
				args[target.name] = target.multiple ? uploaded : uploaded[0]
				sentInputs = attachmentsToMessageInputs(images, blobs)
			} catch (e) {
				sendUserToast(
					`Could not upload the attachments: ${e instanceof Error ? e.message : String(e)}`,
					true
				)
				// The composer already took the draft; without this the turn is simply lost.
				this.#restoreToComposer(options)
				return false
			} finally {
				this.#uploading = false
				this.#manager.isDispatchingTurn = false
			}
		}

		this.#manager.inputMessage = text
		this.#options.onSent?.()
		await this.#manager.sendMessage(
			Object.keys(args).length > 0 || this.#options.additionalInputs?.() ? args : undefined,
			(rowId) => {
				if (!sentInputs) return
				// Drop entries for rows the transcript no longer holds — a conversation
				// switch replaces them all — so this cannot grow past the open chat.
				const live = new Set(this.#manager.messages.map((m) => m.id))
				const kept = Object.fromEntries(
					Object.entries(this.#sentInputs).filter(([id]) => live.has(id))
				)
				this.#sentInputs = { ...kept, [rowId]: sentInputs }
			},
			conversationId
		)
		return true
	}

	/**
	 * Put each attachment in the workspace's object storage and hand back what the
	 * agent reads. The flow runs on a worker, so the bytes have to exist somewhere
	 * the worker can fetch — unlike the copilot, which sends them from the browser.
	 */
	async #uploadAttachments(
		attachments: { name?: string; dataUrl: string; mediaType?: string }[]
	): Promise<S3Attachment[]> {
		const workspace = this.#options.workspace?.()
		if (!workspace) throw new Error('no workspace')
		// One prefix per turn keeps a re-attached filename from overwriting the copy an
		// earlier message still refers to.
		const prefix = `windmill_chat_uploads/${randomUUID()}`
		return Promise.all(
			attachments.map(async (attachment, index) => {
				const filename = attachment.name ?? `attachment-${index + 1}`
				const blob = dataUrlToBlob(attachment.dataUrl, attachment.mediaType)
				const { file_key } = await HelpersService.fileUpload({
					workspace,
					fileKey: `${prefix}/${filename}`,
					contentType: blob.type,
					requestBody: blob
				})
				return { s3: file_key, filename }
			})
		)
	}
	cancel = () => {
		void this.#manager.cancelCurrentJob()
	}
	// Typed off the interface: a Svelte component's own type resolves differently
	// across import specifiers, and the two would then not be assignable.
	#aiChatInput: Parameters<ChatViewHost['setAiChatInput']>[0] = null
	setAiChatInput: ChatViewHost['setAiChatInput'] = (aiChatInput) => {
		this.#aiChatInput = aiChatInput
	}

	// A message typed while the flow is running waits here with its attachments and
	// goes out whole when the run finishes (see flushQueuedMessage).
	queuedMessage = $state('')
	queuedContext = undefined
	queuedImages = $state<AttachedImage[]>([])
	queuedFiles: AttachedTextFile[] = []
	queuedBlobs = $state<AttachedBlob[]>([])
	queueMessage = (
		text: string,
		images: AttachedImage[] = [],
		_context?: unknown,
		_files?: unknown,
		blobs: AttachedBlob[] = []
	) => {
		const trimmed = text.trim()
		if (!trimmed && images.length === 0 && blobs.length === 0) return
		if (trimmed) {
			this.queuedMessage = this.queuedMessage ? `${this.queuedMessage}\n${trimmed}` : trimmed
		}
		this.queuedImages = [...this.queuedImages, ...images]
		this.queuedBlobs = [...this.queuedBlobs, ...blobs]
	}
	/** Put the queued draft back in the composer, attachments included. */
	dequeueMessage = () => {
		const { text, images, blobs } = this.#takeQueue()
		if (!text && images.length === 0 && blobs.length === 0) return
		this.#aiChatInput?.prependText(text, images, [], blobs)
	}
	#takeQueue() {
		const taken = {
			text: this.queuedMessage,
			images: this.queuedImages,
			blobs: this.queuedBlobs
		}
		this.queuedMessage = ''
		this.queuedImages = []
		this.queuedBlobs = []
		return taken
	}
	/** Send whatever was typed during the run. Called once the run settles. */
	flushQueuedMessage = () => {
		// Same rule as sendRequest, read before the queue is drained: a turn with no message
		// cannot run, and taking the queue for it would drop the attachments on the floor.
		if (!this.queuedMessage.trim()) return
		const { text, images, blobs } = this.#takeQueue()
		void this.sendRequest({ instructions: text, images, blobs })
	}
	setComposerStaged = () => {}
	clearComposerStaged = () => {}
	attachmentBytesExcluding = () => 0

	storedImages = () => undefined
	restartGeneration = () => {}
	handleUserQuestionAnswer = () => false
	handleToolConfirmation = () => {}
	// A flow's tools take their arguments from the model, never from a form the reader fills.
	hasPendingRunForm = false
	isRunFormPending = () => false

	mode = undefined
	isSessionChat = false
	supportsModelSettings = false
	supportsMessageEditing = false
	// The turn is a flow run, and an AI agent step refuses one with neither a
	// `user_message` nor manual memory (ai_executor.rs) — so files alone cannot be sent.
	requiresMessageText = true
	// Attachments go to object storage for the worker to read, so a linked folder —
	// a live handle on the user's own disk — has no meaning here.
	get supportsMessageAttachments() {
		return (this.#options.canAttach?.() ?? false) && !!this.#options.attachmentsTarget?.()
	}
	supportsLinkedFolders = false
	attachmentsAsBlobs = true
	// A scalar flow input holds one file; sending more would upload every one and run with
	// the first, leaving the rest orphaned in storage and the transcript claiming otherwise.
	get maxMessageAttachments() {
		return this.#options.attachmentsTarget?.()?.multiple === false ? 1 : undefined
	}
	// What an AI agent step accepts (AI_AGENT_SCHEMA.user_attachments).
	attachmentAccept = 'image/*,application/pdf,.pdf'
	tools = []
	autonomyMode = AIAutonomyMode.DEFAULT
	setAutonomyMode = () => {}
	autoAcceptEditsActive = false
	autoAcceptEditsAvailable = false
	autoAcceptToolConfirmationsAvailable = false
	planModeAvailable = false
	attachedFiles = new AttachedFilesStore()
	artifacts = new SessionArtifactsStore()
}
