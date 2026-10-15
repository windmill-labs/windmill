import { AIAutonomyMode } from './AIChatManager.svelte'
import type { ChatViewHost } from './chatViewHost'
import type { DisplayMessage } from './shared'
import { AttachedFilesStore } from './files/attachedFiles.svelte'
import { SessionArtifactsStore } from './artifacts/artifactsState.svelte'
import type { PersistedArtifact } from './artifacts/artifactsDB'

const noop = () => {}

/**
 * A host over a finished transcript nobody can continue: every affordance that would send,
 * edit, retry or answer is off, so the chat components render it as a record.
 */
export function readOnlyChatViewHost(options: {
	messages: DisplayMessage[]
	workspace: string
	artifacts: PersistedArtifact[]
	openArtifact: (artifactId: string) => void
}): ChatViewHost {
	const artifacts = new SessionArtifactsStore()
	artifacts.artifacts = options.artifacts
	return {
		displayMessages: options.messages,
		messages: [],
		contextTokens: 0,
		operatingWorkspace: options.workspace,
		loading: false,
		sendPending: false,
		runHeldElsewhere: false,
		loadingLabel: undefined,
		compacting: false,
		currentReply: '',
		currentReasoning: '',
		currentReasoningActive: false,
		reasoningHiddenIndicatorLabel: undefined,
		automaticScroll: false,
		enableAutomaticScroll: noop,
		disableAutomaticScroll: noop,

		instructions: '',
		sendInFlight: false,
		sendRequest: async () => false,
		cancel: noop,
		setAiChatInput: noop,
		queuedMessage: '',
		queuedContext: undefined,
		queuedImages: [],
		queuedFiles: [],
		queuedBlobs: [],
		queueMessage: noop,
		dequeueMessage: noop,
		setComposerStaged: noop,
		setComposerHasDraft: noop,
		clearComposerStaged: noop,
		attachmentBytesExcluding: () => 0,

		storedImages: () => undefined,
		retryRequest: noop,
		restartGeneration: noop,
		handleUserQuestionAnswer: () => false,
		handleToolConfirmation: noop,
		hasPendingRunForm: false,
		isRunFormPending: () => false,

		isSessionChat: true,
		supportsModelSettings: false,
		supportsMessageEditing: false,
		supportsMessageAttachments: false,
		requiresMessageText: false,
		supportsLinkedFolders: false,
		attachmentAccept: '',
		attachmentsAsBlobs: false,
		tools: [],
		autonomyMode: AIAutonomyMode.DEFAULT,
		setAutonomyMode: noop,
		autoAcceptEditsActive: false,
		autoAcceptEditsAvailable: false,
		autoAcceptToolConfirmationsAvailable: false,
		planModeAvailable: false,
		attachedFiles: new AttachedFilesStore(),
		artifacts,
		openArtifact: (artifactId) => options.openArtifact(artifactId)
	}
}
