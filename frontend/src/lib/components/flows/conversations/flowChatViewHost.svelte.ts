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
			const fromJob = toolCalls.get(message.job_id)
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

	// What the turn in flight was sent with, until its own row carries a job id.
	#pendingInputs = $state<MessageInputs | undefined>(undefined)

	#messageInputs = new MessageInputsStore(() => this.#options.workspace?.())
	#toolCalls = new ToolCallStore(() => this.#options.workspace?.())

	displayMessages = $derived.by(() => {
		let userIndex = 0
		const showStepNames = this.#showStepNames
		const messages = this.#manager.messages
		// Only the newest user row can be the one the composer just added: every earlier
		// row without a job id predates the column and has no inputs to show.
		let lastUserIndex = -1
		for (let i = messages.length - 1; i >= 0; i--) {
			if (messages[i].message_type === 'user') {
				lastUserIndex = i
				break
			}
		}
		return messages.map((message, i) =>
			toDisplayMessage(
				message,
				message.message_type === 'user' ? userIndex++ : -1,
				showStepNames,
				this.#messageInputs,
				this.#toolCalls,
				message.message_type === 'user' && turnFailed(messages, i),
				i === lastUserIndex ? this.#pendingInputs : undefined
			)
		)
	})

	/** Resend the message at this transcript position, with the inputs it ran with. */
	retryRequest = (messageIndex: number) => {
		const message = this.#manager.messages[messageIndex]
		if (!message || message.message_type !== 'user' || this.loading) return
		void this.sendRequest({ instructions: message.content })
	}
	messages: readonly unknown[] = []
	contextTokens = 0
	loading = $derived.by(
		() => this.#manager.isLoading || this.#manager.isWaitingForResponse || this.#uploading
	)
	loadingLabel = undefined
	compacting = false
	currentReply = ''
	currentReasoning = ''
	currentReasoningActive = false
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

	sendRequest = async (options: ChatSendRequestOptions = {}) => {
		const text = options.instructions?.trim()
		// The conversation's message is the flow's `user_message`, which the manager
		// requires; an attachment-only turn has nothing to send.
		if (!text) return false

		const args = { ...(this.#options.additionalInputs?.() ?? {}) }
		const target = this.#options.attachmentsTarget?.()
		const attachments = [...(options.images ?? []), ...(options.blobs ?? [])]
		this.#pendingInputs = undefined
		if (target && attachments.length > 0) {
			this.#uploading = true
			try {
				const uploaded = await this.#uploadAttachments(attachments)
				args[target.name] = target.multiple ? uploaded : uploaded[0]
				this.#pendingInputs = attachmentsToMessageInputs(options.images ?? [], options.blobs ?? [])
			} catch (e) {
				sendUserToast(
					`Could not upload the attachments: ${e instanceof Error ? e.message : String(e)}`,
					true
				)
				return false
			} finally {
				this.#uploading = false
			}
		}

		this.#manager.inputMessage = text
		this.#options.onSent?.()
		await this.#manager.sendMessage(
			Object.keys(args).length > 0 || this.#options.additionalInputs?.() ? args : undefined
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
		const { text, images, blobs } = this.#takeQueue()
		if (!text) return
		void this.sendRequest({ instructions: text, images, blobs })
	}
	setComposerStaged = () => {}
	clearComposerStaged = () => {}
	attachmentBytesExcluding = () => 0

	storedImages = () => undefined
	restartGeneration = () => {}
	handleUserQuestionAnswer = () => false
	handleToolConfirmation = () => {}

	mode = undefined
	isSessionChat = false
	supportsModelSettings = false
	supportsMessageEditing = false
	// Attachments go to object storage for the worker to read, so a linked folder —
	// a live handle on the user's own disk — has no meaning here.
	get supportsMessageAttachments() {
		return (this.#options.canAttach?.() ?? false) && !!this.#options.attachmentsTarget?.()
	}
	supportsLinkedFolders = false
	attachmentsAsBlobs = true
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
