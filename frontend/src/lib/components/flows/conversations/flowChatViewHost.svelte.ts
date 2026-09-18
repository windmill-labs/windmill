import {
	TurnRunningError,
	type AttachmentUpload,
	type Chat,
	type ChatMessage,
	type ChatState,
	type RunningTurn
} from 'windmill-chat'
import { isBusy, lastTurnFailed, turnFailed } from './flowChatPool'
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
import {
	prefersInstantReveal,
	TypewriterReveal,
	type TypewriterRevealOptions
} from '$lib/components/copilot/chat/typewriterReveal'
import { JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { attachmentLanes } from './messageAttachments'
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
	/** The workspace the transcript's paths resolve against, and a retry reads its run from. */
	workspace?: () => string | undefined
	/** Whether sending is refused right now (a deployment in progress, say). The composer
	 * is disabled on the same condition; this covers the sends the composer does not
	 * make itself: a queued message going out, a retry. */
	sendDisabled?: () => boolean
	/** Flow inputs edited by a control beside the composer, such as the model button. A retry
	 * takes their current value rather than the failed turn's. */
	inputsShownInComposer?: () => string[]
	/** Injectables for tests: the clock and scheduler behind the typewriter pacing. */
	revealOptions?: Pick<TypewriterRevealOptions, 'instant' | 'now' | 'schedule' | 'cancel'>
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
 * The step name says which AI agent step wrote a message, so it only tells the reader
 * anything once the conversation holds more than one. Counted over the transcript rather
 * than over the flow's steps: a conversation outlives edits to the flow, so it can carry
 * labels from a shape the flow no longer has.
 */
export function showsStepNames(messages: readonly ChatMessage[]): boolean {
	return new Set(messages.map((m) => m.stepName).filter(Boolean)).size > 1
}

/** How much of a streaming message is on screen, per lane, in characters. */
export type Revealed = { content: number; reasoning: number }

/** What a display row can only learn beyond the message itself. Every lookup is optional. */
export type DisplayLookups = {
	/** The workspace an attachment's download link points into. */
	workspace?: string
	/** How much of a pending assistant row the pacing has put on screen. */
	revealed?: (message: ChatMessage) => Revealed | undefined
}

/**
 * What a failed tool call returned, as the card's error: the card shows the error in place of
 * the result. A job failure is stored as `{ message, name, stack }`; an MCP failure as plain text.
 */
function toolErrorText(result: unknown): string | undefined {
	if (typeof result === 'string') return result || undefined
	if (result && typeof result === 'object') {
		const message = (result as { message?: unknown }).message
		return typeof message === 'string' ? message : JSON.stringify(result, null, 2)
	}
	return undefined
}

/** The name the worker gives the structured-output tool: suffixed when an agent tool already has it. */
const STRUCTURED_OUTPUT_CALL = /^structured_output(_\d+)?$/

/**
 * `busy`: the latest turn is still running, so a failed tool call is not yet its outcome.
 * `stopped`: ids of the user messages of the turns the reader stopped, as message id or row id.
 * A stopped turn never offers Retry, though Stop leaves its failed tool row or the cancelled
 * flow's failure as its last message.
 */
export function toDisplayMessages(
	messages: readonly ChatMessage[],
	busy = false,
	stopped: ReadonlySet<string> = new Set(),
	lookups: DisplayLookups = {}
): DisplayMessage[] {
	let userIndex = 0
	const stepNames = showsStepNames(messages)
	let latestUser = -1
	for (let i = messages.length - 1; i >= 0 && latestUser < 0; i--) {
		if (messages[i].role === 'user') latestUser = i
	}
	return messages.flatMap((message, i): DisplayMessage[] => {
		switch (message.role) {
			case 'user': {
				const index = userIndex++
				const settled =
					!(busy && i === latestUser) &&
					!stopped.has(message.id) &&
					!(message.serverId && stopped.has(message.serverId))
				const { images, contextElements } = attachmentLanes(lookups.workspace, message.attachments)
				return [
					{
						role: 'user',
						index,
						content: message.content,
						// Drives the shared Retry button.
						error: (settled && turnFailed(messages, i)) || undefined,
						images: images.length > 0 ? images : undefined,
						contextElements: contextElements.length > 0 ? contextElements : undefined
					}
				]
			}
			case 'tool': {
				// The model's call and what the tool sent back, as the stream carried them or the
				// worker stored them on the row. A row stored without them shows its name and job.
				const toolName = message.tool?.name
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
					content: message.content || unfinished || (toolName ? `Running ${toolName}` : ''),
					// Withheld for the copilot's two plan-mode names: `toolName` is what makes
					// ToolExecutionDisplay render a plan card, and an agent tool that happened to
					// share one would silently become one.
					toolName: isPlanCardTool(toolName) ? undefined : toolName,
					parameters,
					result,
					showDetails: parameters !== undefined || result !== undefined,
					error: failed ? (toolErrorText(result) ?? message.content) : unfinished,
					isLoading: message.pending && message.tool?.status === 'running',
					jobId: message.jobId
				}
				// The tool card has no thinking section: the thinking that led to the call reads
				// as a card of its own, just before it.
				return message.reasoning
					? [
							{
								role: 'assistant',
								content: '',
								reasoning: message.reasoning,
								stepName: stepNames ? message.stepName : undefined,
								jobId: message.jobId,
								createdAt: message.createdAt
							},
							call
						]
					: [call]
			}
			default: {
				// The pacing's prefix while the message streams; the whole text once it has
				// settled, or the turn was stopped.
				const revealed = message.pending ? lookups.revealed?.(message) : undefined
				return [
					{
						role: 'assistant',
						content: revealed ? message.content.slice(0, revealed.content) : message.content,
						// Only the message a turn is still writing: a finalized reasoning-only
						// message must not look in progress.
						streaming: message.pending || undefined,
						reasoning: revealed
							? message.reasoning?.slice(0, revealed.reasoning)
							: message.reasoning,
						stepName: stepNames ? message.stepName : undefined,
						jobId: message.jobId,
						createdAt: message.createdAt
					}
				]
			}
		}
	})
}

/** The two paced lanes of one streaming message, and how much of each has been fed in. */
type RevealLanes = {
	content: TypewriterReveal
	reasoning: TypewriterReveal
	fed: Revealed
}

/**
 * Renders a flow run's conversation, as the `windmill-chat` SDK keeps it, through the
 * copilot's chat components. The turn is a flow job rather than an LLM call this host
 * makes, so what it can offer is what the SDK's `Chat` can: a message in, an answer
 * streamed back, Stop. Every copilot-only field is answered with "no" (see ChatViewHost).
 *
 * One host per conversation, living as long as its chat: it outlasts the panel showing it,
 * so a message queued in a conversation still goes out once the reader has moved on.
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

	/** Set by the panel showing this conversation; a message queued here is sent with what
	 * the last panel to show it read. */
	setOptions(options: FlowChatViewHostOptions) {
		this.#options = options
	}

	/** Follows a turn this chat did not start, so what is queued behind it waits for it. */
	resumeTurn = (turn: RunningTurn) => {
		this.#turnDone = this.#chat.resumeTurn(turn)
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
		for (const id of Object.keys(this.#reveals)) this.#dropReveal(id)
	}

	/** The latest `ChatState`, for what the interface reads beyond the seam (paging, loading). */
	get state(): ChatState {
		return this.#state
	}

	#onState(state: ChatState) {
		const previous = this.#state
		this.#state = state
		this.#paceReveals(state.messages)
		const landed = state.messages.filter(
			(m) => m.serverId && this.#stoppedTurns.has(m.id) && !this.#stoppedTurns.has(m.serverId)
		)
		if (landed.length > 0) {
			this.#stoppedTurns = new Set([...this.#stoppedTurns, ...landed.map((m) => m.serverId!)])
		}
		if (previous.conversationId !== state.conversationId) {
			// A conversation opens at its end, whatever the reader was doing in the last one.
			this.#automaticScroll = true
			// The queue was typed into the conversation that just went away; a message sent
			// after the switch would ride out of the wrong one, so it goes back to the composer.
			this.dequeueMessage()
			return
		}
		if (!isBusy(previous.status) && isBusy(state.status)) this.#turnsStarted++
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

	// Smooth streaming. The chat appends each delta to the pending assistant message as it
	// arrives, in the coarse bursts the provider sends; what is shown is a prefix that a
	// typewriter advances per lane, so the bursts read as continuous typing. Plain fields
	// for the pacers, reactive counts for what they have put on screen.
	#reveals: Record<string, RevealLanes> = {}
	#revealed = $state<Record<string, Revealed>>({})

	#paceReveals(messages: readonly ChatMessage[]) {
		const pending = new Set<string>()
		for (let i = 0; i < messages.length; i++) {
			const message = messages[i]
			if (message.role !== 'assistant' || !message.pending) continue
			pending.add(message.id)
			const lanes = (this.#reveals[message.id] ??= this.#newLanes(message.id))
			const content = message.content.slice(lanes.fed.content)
			const reasoning = (message.reasoning ?? '').slice(lanes.fed.reasoning)
			lanes.fed = { content: message.content.length, reasoning: (message.reasoning ?? '').length }
			lanes.content.push(content)
			lanes.reasoning.push(reasoning)
			// A row the turn has moved past — a tool card now follows it — is shown whole:
			// what the pacing still holds belongs above that card, not trickling in under it.
			if (i < messages.length - 1) {
				lanes.content.flush()
				lanes.reasoning.flush()
			}
		}
		for (const id of Object.keys(this.#reveals)) {
			if (!pending.has(id)) this.#dropReveal(id)
		}
	}

	#newLanes(id: string): RevealLanes {
		const options = this.#options.revealOptions ?? {}
		const instant = options.instant ?? prefersInstantReveal()
		const lane = (kind: keyof Revealed) =>
			new TypewriterReveal({
				...options,
				instant,
				onReveal: (chunk) => {
					const current = this.#revealed[id] ?? { content: 0, reasoning: 0 }
					this.#revealed[id] = { ...current, [kind]: current[kind] + chunk.length }
				}
			})
		this.#revealed[id] = { content: 0, reasoning: 0 }
		return {
			content: lane('content'),
			reasoning: lane('reasoning'),
			fed: { content: 0, reasoning: 0 }
		}
	}

	#dropReveal(id: string) {
		this.#reveals[id]?.content.reset()
		this.#reveals[id]?.reasoning.reset()
		delete this.#reveals[id]
		delete this.#revealed[id]
	}

	// Transcript
	displayMessages = $derived.by(() =>
		toDisplayMessages(this.#state.messages, isBusy(this.#state.status), this.#stoppedTurns, {
			workspace: this.#options.workspace?.(),
			revealed: (message) => this.#revealed[message.id]
		})
	)
	/** The user message of each turn stopped in this view, by message id and, once the chat has
	 * read its row, row id: a reopened conversation or an older page rebuilds messages from rows,
	 * so a turn left before its row was read is not recognised there. Only this session knows;
	 * a reload shows the stopped turn as its rows left it. */
	#stoppedTurns = $state.raw<ReadonlySet<string>>(new Set())
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
	/**
	 * `replayInputs` are a failed turn's own run arguments, read back from its job. They
	 * stand in for the composer's current inputs, so a retry runs the turn that failed
	 * rather than a new one wearing its text.
	 */
	sendRequest = async (
		options: ChatSendRequestOptions = {},
		replayInputs?: Record<string, any>
	): Promise<boolean> => {
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
			this.#returnDraft(text, images, blobs)
			return false
		}
		const target = this.#options.attachmentsTarget?.()
		// The inputs modal does not ask for this input, so a required one is enforced here. A
		// replay carries the files its run already has, in `replayInputs`, and attaches none.
		if (!replayInputs && target?.required && images.length === 0 && blobs.length === 0) {
			sendUserToast('This chat needs a file with each message. Attach one to send.', true)
			this.#returnDraft(text, images, blobs)
			return false
		}
		// A replay sends the arguments its run had, attachment references included.
		const inputs = replayInputs ?? { ...(this.#options.additionalInputs?.() ?? {}) }
		// The attachments are this input's only editor: a value stored for it in the inputs
		// modal would otherwise ride along on every message.
		if (target && !replayInputs) delete inputs[target.name]
		// The composer caps files as they are attached, but a queue merged over several turns
		// arrives here as one send, and the chat refuses more than a single-file input holds.
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
		const attachments: AttachmentUpload[] = target
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
		const turn = this.#chat
			.sendMessage(text, {
				inputs: replayInputs ?? (this.#options.additionalInputs?.() ? inputs : undefined),
				attachments,
				attachmentsInput: target
			})
			.catch((e) => {
				if (this.#disposed) return
				if (e instanceof TurnRunningError) {
					// The conversation is still answering a message sent elsewhere: that turn is
					// followed here, and this one waits behind it as if typed during it.
					this.queueMessage(text, images, undefined, undefined, blobs)
					this.resumeTurn(e.turn)
					return
				}
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
				this.#returnDraft(text, images, blobs)
			})
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
		const { messages, status } = this.#state
		const turn = isBusy(status) ? [...messages].reverse().find((m) => m.role === 'user') : undefined
		if (turn) {
			this.#stoppedTurns = new Set(
				[...this.#stoppedTurns, turn.id, turn.serverId].filter((id) => id !== undefined)
			)
		}
		void this.#chat.stop()
	}
	// Typed off the interface: a Svelte component's own type resolves differently
	// across import specifiers, and the two would then not be assignable.
	#aiChatInput: Parameters<ChatViewHost['setAiChatInput']>[0] = null
	setAiChatInput: ChatViewHost['setAiChatInput'] = (aiChatInput) => {
		this.#aiChatInput = aiChatInput
		const { text, images, blobs } = this.#returned
		if (aiChatInput && (text || images.length > 0 || blobs.length > 0)) {
			this.#returned = emptyQueue()
			aiChatInput.prependText(text, images, [], blobs)
		}
	}
	/**
	 * A draft handed back while no composer shows this conversation, for the next one that
	 * does. This host outlives the panel, so a turn that refuses its message after the reader
	 * has moved on has nowhere to put it back until then.
	 */
	#returned = $state<Queue>(emptyQueue())
	#returnDraft(text: string, images: AttachedImage[] = [], blobs: AttachedBlob[] = []) {
		if (this.#aiChatInput) {
			this.#aiChatInput.prependText(text, images, [], blobs)
			return
		}
		this.#returned = {
			text: this.#returned.text ? `${this.#returned.text}\n${text}` : text,
			images: [...this.#returned.images, ...images],
			blobs: [...this.#returned.blobs, ...blobs]
		}
	}

	// One message typed while the turn runs, sent whole with its attachments once the turn
	// settles. Enter again appends a line rather than replacing what waits.
	#queue = $state<Queue>(emptyQueue())
	get queuedMessage(): string {
		return this.#queue.text
	}
	/**
	 * Something typed here has not been sent: waiting for the turn, or handed back by a turn
	 * that refused it while no composer was mounted to take it. Either way this host is the
	 * only place it exists, so nothing may release it.
	 */
	get hasUnsentDraft(): boolean {
		const held = [this.#queue, this.#returned]
		return held.some((q) => q.text !== '' || q.images.length > 0 || q.blobs.length > 0)
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
		this.#returnDraft(text, images, blobs)
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
	/** A retry reading back the turn it is about to replay. Deliberately not part of
	 * `loading`, which renders Stop: there is no run yet to stop. */
	#readingReplayArgs = false
	/** Turns this chat has started, so a retry can tell whether one ran while it read the
	 * arguments, whether or not it is still running. */
	#turnsStarted = 0
	/**
	 * Run the turn at this position again with the arguments its job ran with, not the
	 * composer's current inputs (`inputsShownInComposer` aside). The position is in
	 * `displayMessages`, which holds more entries than the chat's messages. A purged job falls
	 * back to the current inputs.
	 */
	retryRequest = async (messageIndex: number) => {
		const shown = this.displayMessages[messageIndex]
		if (!shown || shown.role !== 'user' || this.loading || this.#readingReplayArgs) return
		const message = this.#state.messages.filter((m) => m.role === 'user')[shown.index]
		if (!message) return
		const conversationId = this.#state.conversationId
		const turnsStarted = this.#turnsStarted
		const workspace = this.#options.workspace?.()
		let replayInputs: Record<string, any> | undefined
		if (message.jobId && workspace) {
			this.#readingReplayArgs = true
			try {
				const original = (await JobService.getJobArgs({ workspace, id: message.jobId })) as
					| Record<string, any>
					| undefined
				// `user_message` is the message itself, passed as the instructions below.
				const { user_message: _sent, ...rest } = original ?? {}
				const current = this.#options.additionalInputs?.() ?? {}
				for (const name of this.#options.inputsShownInComposer?.() ?? []) {
					if (name in current) rest[name] = current[name]
					else delete rest[name]
				}
				replayInputs = rest
			} catch (error) {
				// Only a job that is gone justifies running with other inputs; after a blip or
				// a 500 that would silently run a different turn.
				if ((error as { status?: number })?.status !== 404) {
					sendUserToast('Could not read what that turn ran with. Try again.', true)
					return
				}
			} finally {
				this.#readingReplayArgs = false
			}
		}
		// The reader may have moved on while the arguments were read: to another conversation,
		// where this turn does not belong, or by sending. Any turn started since counts, settled
		// or not: they wrote the chat's latest message, and running this one now would answer
		// something they have moved past.
		if (this.#disposed || this.#state.conversationId !== conversationId) return
		if (this.#turnsStarted !== turnsStarted) {
			sendUserToast('That chat started another turn. Retry once it finishes.', true)
			return
		}
		void this.sendRequest({ instructions: message.content }, replayInputs)
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
	// A single-file flow input takes one attachment per message.
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
