import type {
	ChatSendRequestOptions,
	ChatViewHost
} from '$lib/components/copilot/chat/chatViewHost'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type { ChatMessage, FlowChatManager } from './FlowChatManager.svelte'
import { AIAutonomyMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
import { AttachedFilesStore } from '$lib/components/copilot/chat/files/attachedFiles.svelte'
import { SessionArtifactsStore } from '$lib/components/copilot/chat/artifacts/artifactsState.svelte'

function toDisplayMessage(
	message: ChatMessage,
	userIndex: number,
	showStepNames: boolean
): DisplayMessage {
	switch (message.message_type) {
		case 'user':
			return { role: 'user', index: userIndex, content: message.content }
		case 'tool':
			// Both producers of a tool row — the agent executor and the frontend's own
			// stream parser — write the whole message as a one-line description of the
			// call ("Used web_search tool"). There is no result to reveal, so the row
			// is the label and nothing else. `toolName` stays unset: it drives the
			// copilot's plan-card detection, which a flow step summary must not trip.
			return {
				role: 'tool',
				tool_call_id: message.id,
				content: message.content,
				isLoading: message.loading
			}
		default:
			return {
				role: 'assistant',
				content: message.content,
				streaming: message.streaming,
				stepName: showStepNames ? (message.step_name ?? undefined) : undefined
			}
	}
}

/**
 * Renders a flow run's conversation through the AI session chat components. The
 * turn itself is a flow job, so everything the copilot's own loop owns — context
 * elements, attachments, autonomy, model choice — is absent here, and the chrome
 * driving it hides itself (see ChatViewHost).
 */
export class FlowChatViewHost implements ChatViewHost {
	#manager: FlowChatManager
	#additionalInputs: () => Record<string, any> | undefined

	constructor(
		manager: FlowChatManager,
		additionalInputs: () => Record<string, any> | undefined = () => undefined
	) {
		this.#manager = manager
		this.#additionalInputs = additionalInputs
	}

	// The step name says which AI agent step wrote a message, so it only tells the
	// reader anything once a conversation holds more than one. Counted over the
	// transcript rather than over the flow's current steps: a conversation outlives
	// edits to the flow, so it can carry labels from a shape the flow no longer has.
	#showStepNames = $derived.by(
		() => new Set(this.#manager.messages.map((m) => m.step_name).filter(Boolean)).size > 1
	)

	displayMessages = $derived.by(() => {
		let userIndex = 0
		const showStepNames = this.#showStepNames
		return this.#manager.messages.map((message) =>
			toDisplayMessage(message, message.message_type === 'user' ? userIndex++ : -1, showStepNames)
		)
	})
	messages: readonly unknown[] = []
	contextTokens = 0
	loading = $derived.by(() => this.#manager.isLoading || this.#manager.isWaitingForResponse)
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
	sendRequest = async (options: ChatSendRequestOptions = {}) => {
		const text = options.instructions?.trim()
		if (!text) return false
		this.#manager.inputMessage = text
		await this.#manager.sendMessage(this.#additionalInputs())
		return true
	}
	cancel = () => {
		void this.#manager.cancelCurrentJob()
	}
	setAiChatInput = () => {}

	// A message typed while the flow is running waits here and goes out when the
	// run finishes (see flushQueuedMessage).
	queuedMessage = $state('')
	queuedContext = undefined
	queuedImages = []
	queuedFiles = []
	queueMessage = (text: string) => {
		const trimmed = text.trim()
		if (!trimmed) return
		this.queuedMessage = this.queuedMessage ? `${this.queuedMessage}\n${trimmed}` : trimmed
	}
	dequeueMessage = () => {
		this.queuedMessage = ''
	}
	/** Send whatever was typed during the run. Called once the run settles. */
	flushQueuedMessage = () => {
		const queued = this.queuedMessage
		if (!queued) return
		this.queuedMessage = ''
		void this.sendRequest({ instructions: queued })
	}
	setComposerStaged = () => {}
	clearComposerStaged = () => {}
	attachmentBytesExcluding = () => 0

	storedImages = () => undefined
	retryRequest = () => {}
	restartGeneration = () => {}
	handleUserQuestionAnswer = () => false
	handleToolConfirmation = () => {}

	mode = undefined
	isSessionChat = false
	supportsModelSettings = false
	supportsMessageEditing = false
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
