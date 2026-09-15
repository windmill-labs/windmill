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
import { storedAttachmentName } from './attachmentNames'
import type { AttachedImage } from '$lib/components/copilot/chat/imageUtils'
import type { AttachedTextFile } from '$lib/components/copilot/chat/textFileUtils'
import { HelpersService, JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { randomUUID } from '$lib/utils/uuid'
import { turnFailed } from './turnTranscript'
import {
	argsToMessageInputs,
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
	attachmentsTarget?: () => AttachmentsTarget | undefined
	workspace?: () => string | undefined
	/** Why attaching is off despite the flow taking attachments — no object storage, say.
	 * Undefined while the workspace has not answered: an explanation must not be a guess. */
	attachmentsUnavailable?: () => string | undefined
	/** Flow inputs the composer renders a control of its own for, so a message does not
	 * repeat them as context chips. */
	inputsShownInComposer?: () => string[]
	/** The flow's input schema, which says which of a run's arguments are secret. */
	inputsSchema?: () => { properties?: Record<string, any> } | undefined
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
			// What the turn ran with — the message row itself keeps only the text. Renders
			// through the same lanes the copilot uses.
			// What this tab sent wins wherever it has it, and the job answers for the rest:
			// a row read back from the server on a later visit, a conversation reopened.
			// The row is named with its job as soon as the run starts, so a send that kept
			// nothing would switch lanes mid-run and fetch arguments it had just handed over.
			const { images, contextElements } =
				pendingInputs ??
				(message.job_id ? inputs.get(message.job_id) : { images: [], contextElements: [] })
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
			// The same three details reach a row from one of two places, never both: the row
			// itself for a tool with no job of its own, and for one still streaming; the
			// tool's own job otherwise.
			const fromRow: ToolCallDetails = {
				toolName: message.tool_name,
				parameters: parseToolPayload(message.tool_arguments),
				result: parseToolPayload(message.tool_result)
			}
			// A row that carries its own call must not ask a job for it: an MCP tool runs
			// inside the agent's job and names it, so the answer would be the agent's own
			// arguments and result rather than the tool's. (It also saves a fetch per row
			// when a conversation opens.)
			const carriesItsOwnCall = fromRow.parameters !== undefined || fromRow.result !== undefined
			const fromJob = carriesItsOwnCall ? EMPTY_TOOL_CALL : toolCalls.get(message.job_id)
			const toolName = fromRow.toolName ?? fromJob.toolName
			const parameters = fromRow.parameters ?? fromJob.parameters
			const result = failed ? undefined : (fromRow.result ?? fromJob.result)
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
 * Renders a flow run's conversation through the AI session chat components. The turn is a
 * flow job rather than an LLM call this host makes, so what it can offer is whatever it can
 * write back into the run's arguments — each field below says for itself (see ChatViewHost).
 */
export class FlowChatViewHost implements ChatViewHost {
	#manager: FlowChatManager
	#options: FlowChatViewHostOptions

	constructor(manager: FlowChatManager, options: FlowChatViewHostOptions = {}) {
		this.#manager = manager
		this.#options = options
		// The queue belongs to the chat it was typed into, and that chat's run can finish
		// while the reader is in another one — so the manager says which turn settled rather
		// than the composer watching the open chat.
		manager.onTurnSettled = (conversationId) => this.flushQueuedMessage(conversationId)
		// The sidebar marks a chat with something waiting to go out; the queue lives here.
		manager.hasQueuedMessage = (conversationId) => !!this.#queues[conversationId]?.text.trim()
	}

	// The step name says which AI agent step wrote a message, so it only tells the
	// reader anything once a conversation holds more than one. Counted over the
	// transcript rather than over the flow's current steps: a conversation outlives
	// edits to the flow, so it can carry labels from a shape the flow no longer has.
	#showStepNames = $derived.by(
		() => new Set(this.#manager.messages.map((m) => m.step_name).filter(Boolean)).size > 1
	)

	// The inputs a turn was sent with, by the id of the row the composer added for it.
	// The row is named with its job once the run starts, so the job could answer this too —
	// but that is a fetch for arguments this tab just sent, and one that fails would blank
	// the row for the rest of the session. Every send records, not only one carrying files:
	// a row with no entry switches to the job lane the moment it is named, which renders it
	// empty for the length of a round trip.
	#sentInputs = $state<Record<string, MessageInputs>>({})

	#messageInputs = new MessageInputsStore(
		() => this.#options.workspace?.(),
		() => this.#options.inputsSchema?.(),
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
	 * Run the turn at this transcript position again, as it ran the first time: its own
	 * arguments, read back from its job, rather than whatever the composer holds now. The
	 * row shows the attachments and inputs it ran with, so a retry that quietly used today's
	 * settings would run something other than what the reader is looking at.
	 *
	 * The composer's own controls are the exception. What they edit — the provider, the
	 * model, the thinking level — is on screen beside the transcript rather than on the row,
	 * and a model that cannot answer is one of the likelier reasons a turn failed. Changing
	 * it and pressing Retry has to run the new one, or the retry fails the same way with no
	 * sign of why.
	 *
	 * A job that has been purged can no longer say what it ran with; the composer is then
	 * the only account left, and the row shows nothing either, so the two still agree.
	 */
	retryRequest = async (messageIndex: number) => {
		const message = this.#manager.messages[messageIndex]
		if (!message || message.message_type !== 'user' || this.loading || this.#readingReplayArgs)
			return
		// The turn belongs to the chat it was clicked in. Read before the fetch below, since
		// the reader can select another conversation while it runs — the same reason the
		// upload path pins it.
		const conversationId = this.#manager.selectedConversationId
		const workspace = this.#options.workspace?.()
		let replayArgs: Record<string, any> | undefined
		if (message.job_id && workspace) {
			this.#readingReplayArgs = true
			try {
				const original = (await JobService.getJobArgs({
					workspace,
					id: message.job_id
				})) as Record<string, any>
				// `user_message` is the message itself, passed as the instructions below.
				const { user_message: _sent, ...rest } = original ?? {}
				const composerOwned = this.#options.inputsShownInComposer?.() ?? []
				const current = this.#options.additionalInputs?.() ?? {}
				for (const name of composerOwned) {
					if (name in current) rest[name] = current[name]
					else delete rest[name]
				}
				replayArgs = rest
			} catch (error) {
				// Only a job that is gone justifies running something else. Anything else —
				// a network blip, a 500 — would substitute a different turn silently, which
				// is the whole thing this guards against.
				if ((error as { status?: number })?.status !== 404) {
					sendUserToast('Could not read what that turn ran with. Try again.', true)
					return
				}
			} finally {
				this.#readingReplayArgs = false
			}
		}
		// A send typed while the arguments were being read has already started a turn here,
		// and `sendRequest` would refuse this one by handing its text to the composer as if
		// the reader had typed it.
		if (conversationId && this.#manager.isConversationBusy(conversationId)) {
			sendUserToast('That chat started another turn. Retry once it finishes.', true)
			return
		}
		void this.sendRequest({ instructions: message.content, conversationId }, replayArgs)
	}
	messages: readonly unknown[] = []
	contextTokens = 0
	operatingWorkspace = $derived.by(() => this.#options.workspace?.())
	/** A retry reading back the turn it is about to replay. Read only by `retryRequest`, so a
	 * plain field: nothing renders from it. Deliberately not part of `loading`, which renders
	 * Stop — there is no run yet to stop, and Stop would cancel the failed turn's own job and
	 * take the chat's queue back. A normal send in this window still gets ahead of the retry;
	 * the toast below is what says so. */
	#readingReplayArgs = false
	loading = $derived.by(
		() =>
			this.#manager.isLoading ||
			this.#manager.isWaitingForResponse ||
			this.#manager.isDispatchingTurn
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
	/**
	 * Chats whose in-flight send the reader stopped before it had a job to cancel. Read only
	 * by the send that set it aside, so a plain Set: nothing renders from this.
	 */
	#abortedSends = new Set<string>()

	/**
	 * Give a spent draft back after a send that did not run. The composer took it before
	 * calling, so something has to.
	 *
	 * The composer belongs to whichever chat is on screen, so a turn that was not the open
	 * one's goes back to its own queue instead: a flush fires when *its* chat's run settles,
	 * which with turns running side by side can be while the reader is somewhere else, and
	 * dropping the text into the composer there would put it in the wrong conversation.
	 */
	#restoreToComposer(options: ChatSendRequestOptions) {
		const conversationId = options.conversationId ?? this.#manager.selectedConversationId
		// Back to the queue whenever that chat has one, open or not. The composer is the
		// right home for a spent draft only while nothing is waiting behind it: anything
		// queued was typed later, and that chat's next settled run sends its queue whole —
		// so putting the older draft in the composer would run the two out of order.
		const waiting = conversationId ? this.#queueOf(conversationId) : undefined
		const queueHasMore =
			!!waiting && (!!waiting.text.trim() || waiting.images.length > 0 || waiting.blobs.length > 0)
		if (
			conversationId &&
			(queueHasMore || conversationId !== this.#manager.selectedConversationId)
		) {
			this.#enqueue(
				conversationId,
				options.instructions ?? '',
				options.images ?? [],
				options.blobs ?? [],
				'front'
			)
			return
		}
		this.#aiChatInput?.prependText(
			options.instructions ?? '',
			options.images ?? [],
			[],
			options.blobs ?? []
		)
	}

	/**
	 * `replayArgs` are a failed turn's own run arguments, read back from its job. They stand
	 * in for the composer's current inputs — attachments included — so a retry runs the turn
	 * that failed rather than a new one wearing its text.
	 */
	sendRequest = async (options: ChatSendRequestOptions = {}, replayArgs?: Record<string, any>) => {
		const text = options.instructions?.trim() ?? ''
		const args = { ...(replayArgs ?? this.#options.additionalInputs?.() ?? {}) }
		const target = this.#options.attachmentsTarget?.()
		// Where the paperclip is the input's editor, the stored settings have no say over it:
		// a value saved while the modal owned it — before this workspace had object storage —
		// would otherwise ride along on every later message. Attachments are set below or not
		// at all. Where the modal still owns it, what the reader typed there stands. A replay
		// is the exception: its attachments are the ones the failed turn ran with.
		if (target && this.supportsMessageAttachments && !replayArgs) {
			delete args[target.name]
		}
		let images = options.images ?? []
		let blobs = options.blobs ?? []
		// The composer refuses an attachment-only send (requiresMessageText), so this is
		// the same rule at the other end: nothing runs without a message.
		if (!text) return false
		// And nothing runs into a conversation belonging to the other surface: the composer
		// is shut for it, but a queued turn could have been written before it was opened.
		// Reads the open conversation, which is the turn's own except for a flush into a
		// background chat — reachable only if a surface ever both runs turns in parallel and
		// lists both kinds of chat. No surface does today; one that did would need this to
		// take `conversationId`.
		const wrongKind = this.#manager.wrongKindReason
		if (wrongKind) {
			sendUserToast(wrongKind, true)
			this.#restoreToComposer(options)
			return false
		}
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
		// Settled before the upload below, and created here when there is none yet rather than
		// left to `sendMessage` afterwards: everything that has to name this turn while it
		// uploads — its busy state, a Stop, a message typed behind it — needs an id to name it
		// by, and a send with none had no way to be marked, stopped or queued against.
		// Read before the upload for the same reason: the reader can pick another chat while
		// it runs, and the turn belongs to the one they sent it from.
		const conversationId =
			options.conversationId ??
			this.#manager.selectedConversationId ??
			(await this.#manager.createConversation({ clearMessages: false }))
		// Whether the composer's own files are what this turn's attachment input holds. A
		// replay's do not: its attachments are the ones its job already has.
		let attachedLocally = false
		if (target && attachments.length > 0) {
			// Named, not "the open chat": the reader can switch while the upload runs, and
			// where turns run in parallel the other chat has its own status to keep.
			this.#manager.setDispatching(conversationId, true)
			this.#abortedSends.delete(conversationId)
			try {
				const uploaded = await this.#uploadAttachments(attachments)
				args[target.name] = target.multiple ? uploaded : uploaded[0]
			} catch (e) {
				sendUserToast(
					`Could not upload the attachments: ${e instanceof Error ? e.message : String(e)}`,
					true
				)
				// The composer already took the draft; without this the turn is simply lost. The
				// id is the one captured before the upload, not whatever is open now: the reader
				// can switch while it runs, and the draft belongs to the chat they sent it from.
				this.#restoreToComposer({ ...options, conversationId })
				return false
			} finally {
				this.#manager.setDispatching(conversationId, false)
			}
			// Stop pressed while the upload ran has no job to cancel yet, so it is honoured
			// here — the run has not started, and starting it now would execute a message the
			// reader already took back.
			if (this.#abortedSends.delete(conversationId)) {
				this.#restoreToComposer({ ...options, conversationId })
				return false
			}
			attachedLocally = true
		}
		const sentInputs = this.#describeSentInputs(args, images, blobs, attachedLocally)

		this.#manager.inputMessage = text
		const started = await this.#manager.sendMessage(
			Object.keys(args).length > 0 || replayArgs || this.#options.additionalInputs?.()
				? args
				: undefined,
			(rowId) => {
				if (!sentInputs) return
				// Keep entries for rows some conversation still holds, not just the open one:
				// a queued message flushes into the chat it was typed in, which by then need
				// not be on screen, and pruning against the open chat would drop its chips.
				const live = this.#manager.liveRowIds
				const kept = Object.fromEntries(
					Object.entries(this.#sentInputs).filter(([id]) => live.has(id))
				)
				this.#sentInputs = { ...kept, [rowId]: sentInputs }
			},
			conversationId
		)
		if (!started) {
			// The upload succeeded and the run did not, so the composer's draft was spent on
			// nothing. What the reader wrote comes back, to the chat it was written in rather
			// than the one open by now.
			//
			// The uploaded objects are deliberately left in place. Deleting them is itself a
			// request that can fail, on a path that is already failing, and a resend uploads
			// its own under a fresh prefix — so a lost send costs one prefix, not a growing
			// number. The workspace's own storage retention is what collects them.
			this.#restoreToComposer({ ...options, conversationId })
			return false
		}
		return true
	}

	/**
	 * The chips a row shows for the turn just sent, built from the arguments going out
	 * through the same split `argsToMessageInputs` makes of a job's arguments.
	 *
	 * Files the composer attached are the exception: the data URLs are still in hand and
	 * render with no network, where the args hold S3 references `argsToMessageInputs` would
	 * turn into `download_s3_file` links for bytes this tab already has. So that one input
	 * is described from what was attached and every other from the args. A replay attached
	 * nothing locally and is described entirely from the args, which are its own job's.
	 */
	#describeSentInputs(
		args: Record<string, any>,
		images: AttachedImage[],
		blobs: AttachedBlob[],
		attachedLocally: boolean
	): MessageInputs | undefined {
		const workspace = this.#options.workspace?.()
		// Without one there is no way to build a file's link, so the job lane answers instead.
		if (!workspace) return undefined
		const target = this.#options.attachmentsTarget?.()
		const shownElsewhere = new Set(this.#options.inputsShownInComposer?.() ?? [])
		if (attachedLocally && target) shownElsewhere.add(target.name)
		const fromArgs = argsToMessageInputs(
			workspace,
			args,
			this.#options.inputsSchema?.(),
			shownElsewhere
		)
		const attached = attachedLocally
			? attachmentsToMessageInputs(images, blobs)
			: { images: [], contextElements: [] }
		return {
			images: [...attached.images, ...fromArgs.images],
			contextElements: [...attached.contextElements, ...fromArgs.contextElements]
		}
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
		// A prefix per turn, and a segment per attachment inside it. The turn's prefix keeps a
		// re-attached filename off the copy an earlier message still points at; the segment
		// does the same within one turn, where two files can arrive under one name and would
		// otherwise race to a single key and leave the agent reading one of them twice. The
		// name itself stays the last segment, so anything that reads a name off the key
		// still sees what the reader attached.
		const prefix = `windmill_chat_uploads/${randomUUID()}`
		return Promise.all(
			attachments.map(async (attachment, index) => {
				const blob = dataUrlToBlob(attachment.dataUrl, attachment.mediaType)
				const filename = storedAttachmentName(
					attachment.name ?? `attachment-${index + 1}`,
					blob.type
				)
				const { file_key } = await HelpersService.fileUpload({
					workspace,
					fileKey: `${prefix}/${index}/${filename}`,
					contentType: blob.type,
					requestBody: blob
				})
				return { s3: file_key, filename }
			})
		)
	}

	cancel = () => {
		// Stop means stop: what was typed during the run goes back to the composer rather
		// than waiting there to go out after some later turn settles.
		this.dequeueMessage()
		// A send still uploading has no job to cancel; it reads this once the upload lands.
		const open = this.#manager.selectedConversationId
		if (open) this.#abortedSends.add(open)
		void this.#manager.cancelCurrentJob()
	}
	// Typed off the interface: a Svelte component's own type resolves differently
	// across import specifiers, and the two would then not be assignable.
	#aiChatInput: Parameters<ChatViewHost['setAiChatInput']>[0] = null
	setAiChatInput: ChatViewHost['setAiChatInput'] = (aiChatInput) => {
		this.#aiChatInput = aiChatInput
	}

	// A message typed while a chat is running waits here with its attachments and goes out
	// whole when that chat's run finishes (see flushQueuedMessage). Held per conversation:
	// turns run side by side, so a message typed into one must not ride out of another.
	#queues = $state<
		Record<string, { text: string; images: AttachedImage[]; blobs: AttachedBlob[] }>
	>({})
	queuedContext = undefined
	queuedFiles: AttachedTextFile[] = []

	#queueOf(conversationId: string | undefined) {
		return (
			(conversationId ? this.#queues[conversationId] : undefined) ?? {
				text: '',
				images: [] as AttachedImage[],
				blobs: [] as AttachedBlob[]
			}
		)
	}
	get queuedMessage(): string {
		return this.#queueOf(this.#manager.selectedConversationId).text
	}
	get queuedImages(): AttachedImage[] {
		return this.#queueOf(this.#manager.selectedConversationId).images
	}
	get queuedBlobs(): AttachedBlob[] {
		return this.#queueOf(this.#manager.selectedConversationId).blobs
	}

	queueMessage = (
		text: string,
		images: AttachedImage[] = [],
		_context?: unknown,
		_files?: unknown,
		blobs: AttachedBlob[] = []
	) => {
		this.#enqueue(this.#manager.selectedConversationId, text, images, blobs)
	}

	/**
	 * Add to what a chat already has waiting, rather than replacing it: the reader can type
	 * again while a flush of the previous queue is still uploading, and that second message
	 * is in the queue by the time a failed flush hands the first one back.
	 */
	#enqueue(
		conversationId: string | undefined,
		text: string,
		images: AttachedImage[],
		blobs: AttachedBlob[],
		/** Where this belongs in what is already waiting. A draft handed back by a send that
		 * did not run was typed before anything queued behind it, and goes back in front. */
		at: 'end' | 'front' = 'end'
	) {
		if (!conversationId) return
		const trimmed = text.trim()
		if (!trimmed && images.length === 0 && blobs.length === 0) return
		const queue = this.#queueOf(conversationId)
		const joined = !trimmed
			? queue.text
			: !queue.text
				? trimmed
				: at === 'front'
					? `${trimmed}\n${queue.text}`
					: `${queue.text}\n${trimmed}`
		this.#queues[conversationId] = {
			text: joined,
			images: at === 'front' ? [...images, ...queue.images] : [...queue.images, ...images],
			blobs: at === 'front' ? [...blobs, ...queue.blobs] : [...queue.blobs, ...blobs]
		}
	}
	/** Put the queued draft back in the composer, attachments included. */
	dequeueMessage = () => {
		const { text, images, blobs } = this.#takeQueue(this.#manager.selectedConversationId)
		if (!text && images.length === 0 && blobs.length === 0) return
		this.#aiChatInput?.prependText(text, images, [], blobs)
	}
	#takeQueue(conversationId: string | undefined) {
		const taken = this.#queueOf(conversationId)
		if (conversationId) delete this.#queues[conversationId]
		return taken
	}
	/** Send whatever was typed during the run. Called once that chat's run settles. */
	flushQueuedMessage = (conversationId = this.#manager.selectedConversationId) => {
		// Same rule as sendRequest, read before the queue is drained: a turn with no message
		// cannot run, and taking the queue for it would drop the attachments on the floor.
		if (!this.#queueOf(conversationId).text.trim()) return
		const { text, images, blobs } = this.#takeQueue(conversationId)
		void this.sendRequest({ instructions: text, images, blobs, conversationId })
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
	// The input's shape alone: whether this chat takes attachments at all is a fact about the
	// flow, not about the workspace. Object storage decides whether it can right now, which is
	// `attachmentsUnavailableReason` — a state on the control rather than a reason to move the
	// input to the modal, where a file picker would be just as unable to upload.
	get supportsMessageAttachments() {
		return !!this.#options.attachmentsTarget?.()
	}
	get attachmentsUnavailableReason() {
		return this.#options.attachmentsUnavailable?.()
	}
	supportsLinkedFolders = false
	attachmentsAsBlobs = true
	// A scalar flow input holds one file; sending more would upload every one and run with
	// the first, leaving the rest orphaned in storage and the transcript claiming otherwise.
	get maxMessageAttachments() {
		return this.#options.attachmentsTarget?.()?.multiple === false ? 1 : undefined
	}
	// What a provider actually takes. Anthropic's document block accepts base64
	// `application/pdf` and nothing else, so the wider set `is_document_mime`
	// (windmill-ai/src/ai_types.rs) claims — csv, html, plain, docx, xlsx — is rejected with a
	// 400 rather than read. Widen this only alongside a worker that inlines text as text.
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
