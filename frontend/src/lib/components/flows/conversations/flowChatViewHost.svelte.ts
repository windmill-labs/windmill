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
import {
	prefersInstantReveal,
	TypewriterReveal,
	type TypewriterRevealOptions
} from '$lib/components/copilot/chat/typewriterReveal'
import { JobService, type FlowModule } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { ToolCallStore, type ToolCallDetails } from './toolCallContext.svelte'
import {
	argsToMessageInputs,
	attachmentsToMessageInputs,
	MessageInputsStore,
	type MessageInputs
} from './messageInputContext.svelte'

export type FlowChatViewHostOptions = {
	/** The flow inputs sent next to `user_message` with every turn. */
	additionalInputs?: () => Record<string, any> | undefined
	/** The workspace the transcript's paths resolve against, and the jobs are read from. */
	workspace?: () => string | undefined
	/** Whether sending is refused right now (a deployment in progress, say). The composer
	 * is disabled on the same condition; this covers the sends the composer does not
	 * make itself: a queued message going out, a retry. */
	sendDisabled?: () => boolean
	/** The flow's modules, which say which of a tool job's arguments the model supplied. */
	flowModules?: () => FlowModule[] | undefined
	/** The flow's input schema, which says which of a run's arguments are secret. */
	inputsSchema?: () => { properties?: Record<string, any> } | undefined
	/** Flow inputs edited by a control beside the composer rather than the Inputs modal. No
	 * surface has one yet. A message does not repeat them as chips, and a retry takes their
	 * current value rather than the failed turn's. */
	inputsShownInComposer?: () => string[]
	/** Injectables for tests: the clock and scheduler behind the typewriter pacing. */
	revealOptions?: Pick<TypewriterRevealOptions, 'instant' | 'now' | 'schedule' | 'cancel'>
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
 * A failed tool's result as the line the card shows for it: the worker stores what the
 * tool failed with, a string for the failures it words itself and a structured error for
 * one a job reported. Anything else says nothing about the failure.
 */
function asErrorText(result: unknown): string | undefined {
	if (typeof result === 'string') return result
	const message = (result as any)?.error?.message ?? (result as any)?.message
	return typeof message === 'string' ? message : undefined
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

const EMPTY_TOOL_CALL: ToolCallDetails = {}
const EMPTY_INPUTS: MessageInputs = { images: [], contextElements: [] }

/** What a display row can only learn beyond the message itself. Every lookup is optional. */
export type DisplayLookups = {
	/** A tool row's call, from the job it names. */
	toolCall?: (jobId: string | undefined) => ToolCallDetails
	/** What a user row ran with. */
	inputs?: (message: ChatMessage) => MessageInputs
	/** How much of a pending assistant row the pacing has put on screen. */
	revealed?: (message: ChatMessage) => Revealed | undefined
}

export function toDisplayMessages(
	messages: readonly ChatMessage[],
	lookups: DisplayLookups = {}
): DisplayMessage[] {
	let userIndex = 0
	const stepNames = showsStepNames(messages)
	return messages.map((message, i): DisplayMessage => {
		switch (message.role) {
			case 'user': {
				const { images, contextElements } = lookups.inputs?.(message) ?? EMPTY_INPUTS
				return {
					role: 'user',
					index: userIndex++,
					content: message.content,
					// Drives the shared Retry button.
					error: turnFailed(messages, i) || undefined,
					images: images.length > 0 ? images : undefined,
					contextElements: contextElements.length > 0 ? contextElements : undefined
				}
			}
			case 'tool': {
				const failed = message.success === false
				// A row carrying its own call (one that streamed) must not ask a job for it: an
				// MCP tool runs inside the agent's job and names it, so the job would answer
				// with the agent's arguments and result rather than the tool's.
				const fromRow: ToolCallDetails = {
					toolName: message.tool?.name,
					parameters: parseToolPayload(message.tool?.arguments),
					result: parseToolPayload(message.tool?.result)
				}
				const carriesItsOwnCall = fromRow.parameters !== undefined || fromRow.result !== undefined
				const fromJob =
					carriesItsOwnCall || !lookups.toolCall ? EMPTY_TOOL_CALL : lookups.toolCall(message.jobId)
				const toolName = fromRow.toolName ?? fromJob.toolName
				const parameters = fromRow.parameters ?? fromJob.parameters
				const result = fromRow.result ?? fromJob.result
				return {
					role: 'tool',
					tool_call_id: message.id,
					// The card's header is the row's text, which the server only words once the
					// tool has returned; until then the row says what is running.
					content: message.content || (toolName ? `Running ${toolName}` : ''),
					// Withheld for the copilot's two plan-mode names: `toolName` is what makes
					// ToolExecutionDisplay render a plan card, and an agent tool that happened to
					// share one would silently become one.
					toolName: isPlanCardTool(toolName) ? undefined : toolName,
					parameters,
					result,
					showDetails: parameters !== undefined || result !== undefined,
					// What the tool failed with, from its result, else the row's own sentence.
					error: failed ? (asErrorText(result) ?? message.content) : undefined,
					isLoading: message.pending && message.tool?.status === 'running'
				}
			}
			default: {
				// The pacing's prefix while the message streams; the whole text once it has
				// settled, or the turn was stopped.
				const revealed = message.pending ? lookups.revealed?.(message) : undefined
				return {
					role: 'assistant',
					content: revealed ? message.content.slice(0, revealed.content) : message.content,
					// Only the message a turn is still writing: a finalized reasoning-only
					// message must not look in progress.
					streaming: message.pending || undefined,
					reasoning: revealed ? message.reasoning?.slice(0, revealed.reasoning) : message.reasoning,
					stepName: stepNames ? message.stepName : undefined,
					jobId: message.jobId,
					createdAt: message.createdAt
				}
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

	// The inputs each send went out with, by the id of the user row the chat appended for it.
	// Every send records: a row with no entry falls to the job lane once it is named, and
	// renders empty for a round trip spent fetching what this tab just sent.
	#sentInputs = $state<Record<string, MessageInputs>>({})

	#messageInputs = new MessageInputsStore(
		() => this.#options.workspace?.(),
		() => this.#options.inputsSchema?.(),
		() => new Set(this.#options.inputsShownInComposer?.() ?? [])
	)
	#toolCalls = new ToolCallStore(
		() => this.#options.workspace?.(),
		() => this.#options.flowModules?.()
	)

	// Transcript
	displayMessages = $derived.by(() =>
		toDisplayMessages(this.#state.messages, {
			toolCall: (jobId) => this.#toolCalls.get(jobId),
			// What this tab sent wins wherever it has it, and the job answers for the rest: a
			// row read back from the server on a later visit, a conversation reopened.
			inputs: (message) =>
				this.#sentInputs[message.id] ??
				(message.jobId ? this.#messageInputs.get(message.jobId) : EMPTY_INPUTS),
			revealed: (message) => this.#revealed[message.id]
		})
	)
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
		const inputs = replayInputs ?? this.#options.additionalInputs?.()
		const before = this.#state.messages.length
		// A run that fails is reported through the chat's `onError` and as a failed message;
		// the promise itself only rejects when the chat refuses the turn outright, and the
		// text is then handed back rather than dropped.
		const turn = this.#chat
			.sendMessage(text, { inputs })
			.catch(() => this.#aiChatInput?.prependText(text))
		this.#turnDone = turn
		// The chat appends the user row before it awaits anything, so the row it added for
		// this send is the last one now, and its inputs are recorded against it.
		const added = this.#state.messages[before]
		if (added?.role === 'user' && added.pending && this.#state.messages.length === before + 1) {
			this.#recordSentInputs(added.id, inputs, options.images ?? [])
		}
		await turn
		return true
	}

	/**
	 * The chips a row shows for the turn just sent, through the same split a job's
	 * arguments get, so the row looks the same before and after a reload. Images the
	 * composer attached are described from what was attached: their data URLs are still
	 * in hand and render with no fetch.
	 */
	#recordSentInputs(
		rowId: string,
		inputs: Record<string, any> | undefined,
		images: AttachedImage[]
	) {
		const workspace = this.#options.workspace?.()
		// Without one there is no way to build a file's link, so the job lane answers instead.
		if (!workspace) return
		const fromArgs = argsToMessageInputs(
			workspace,
			inputs,
			this.#options.inputsSchema?.(),
			new Set(this.#options.inputsShownInComposer?.() ?? [])
		)
		const attached = attachmentsToMessageInputs(images, [])
		// Entries for rows the transcript still holds, so a long session does not keep every
		// turn it ever sent.
		const live = new Set(this.#state.messages.map((m) => m.id))
		const kept = Object.fromEntries(Object.entries(this.#sentInputs).filter(([id]) => live.has(id)))
		this.#sentInputs = {
			...kept,
			[rowId]: {
				images: [...attached.images, ...fromArgs.images],
				contextElements: [...attached.contextElements, ...fromArgs.contextElements]
			}
		}
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
	/** A retry reading back the turn it is about to replay. Deliberately not part of
	 * `loading`, which renders Stop: there is no run yet to stop. */
	#readingReplayArgs = false
	/**
	 * Run the turn at this position again with the arguments its job ran with, which are the
	 * ones its row shows, not the composer's current inputs (`inputsShownInComposer` aside).
	 * A purged job shows nothing on the row either, so it falls back to the current inputs.
	 */
	retryRequest = async (messageIndex: number) => {
		const message = this.#state.messages[messageIndex]
		if (!message || message.role !== 'user' || this.loading || this.#readingReplayArgs) return
		const conversationId = this.#state.conversationId
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
		// The reader may have moved on while the arguments were read: to another
		// conversation, where this turn does not belong, or by sending, which started a turn
		// that `sendRequest` would queue this one behind as if it had been typed.
		if (this.#disposed || this.#state.conversationId !== conversationId) return
		if (this.loading) {
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
