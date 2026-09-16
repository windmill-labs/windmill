<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Loader2, MessageCircle, Settings2 } from 'lucide-svelte'
	import AIChatDisplay from '$lib/components/copilot/chat/AIChatDisplay.svelte'
	import { setChatViewHost } from '$lib/components/copilot/chat/chatViewHost'
	import { FlowChatViewHost } from './flowChatViewHost.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { emptyString, type DynamicInput } from '$lib/utils'
	import { onDestroy, tick, untrack } from 'svelte'
	import type { Chat } from 'windmill-chat'
	import type { FlowModule } from '$lib/gen'
	import { useWorkspaceStorageConfigured } from '$lib/components/inputTransformEnv.svelte'
	import {
		attachmentsTargetFor,
		isEmptyAgentChatInputValue,
		PER_TURN_AGENT_CHAT_INPUT_KEY,
		resolveAgentChatInputs
	} from './agentAttachmentInput'

	interface Props {
		chat: Chat
		deploymentInProgress?: boolean
		additionalInputsSchema?: Record<string, any>
		/** The flow's steps, read for the AI agent inputs the composer drives itself. */
		flowModules?: FlowModule[]
		path: string
		workspace?: string
		/** The flow's description, shown under the empty transcript's prompt. */
		description?: string
		wideLayout?: boolean
	}

	let {
		chat,
		deploymentInProgress = false,
		additionalInputsSchema,
		flowModules = undefined,
		path,
		workspace = undefined,
		description = undefined,
		wideLayout = false
	}: Props = $props()

	// Derive helperScript for dynamic inputs from schema
	const dynamicInputHelperScript = $derived.by((): DynamicInput.HelperScript | undefined => {
		const dynCode = additionalInputsSchema?.['x-windmill-dyn-select-code']
		const dynLang = additionalInputsSchema?.['x-windmill-dyn-select-lang']
		if (dynCode && dynLang) {
			return { source: 'inline', code: dynCode, lang: dynLang }
		}
		return undefined
	})

	// The composer's attachments feed this input, and the paperclip is its whole editor.
	const attachmentsTarget = $derived.by(() => {
		const target = attachmentsTargetFor(
			resolveAgentChatInputs(flowModules, additionalInputsSchema).find(
				(input) => input.key === PER_TURN_AGENT_CHAT_INPUT_KEY
			)
		)
		const required: unknown = additionalInputsSchema?.required
		return target && Array.isArray(required) && required.includes(target.name)
			? { ...target, required: true }
			: target
	})
	// Uploading needs the workspace's object storage; without one the `+` is drawn disabled
	// saying so, since the modal could not upload either.
	const workspaceStorage = useWorkspaceStorageConfigured(() => workspace)

	// What the Configure-inputs modal asks for: every flow input the composer does not edit
	// itself. A stored value for the promoted input is left where it is; the host drops it
	// from what a turn sends.
	const modalSchema = $derived.by(() => {
		if (!additionalInputsSchema) return undefined
		const promoted = attachmentsTarget?.name
		if (!promoted) return additionalInputsSchema
		const properties = Object.fromEntries(
			Object.entries(additionalInputsSchema.properties ?? {}).filter(([key]) => key !== promoted)
		)
		if (Object.keys(properties).length === 0) return undefined
		const required: string[] = Array.isArray(additionalInputsSchema.required)
			? additionalInputsSchema.required
			: []
		return {
			...additionalInputsSchema,
			properties,
			required: required.filter((key) => key !== promoted)
		}
	})

	// LocalStorage helpers
	const STORAGE_KEY_PREFIX = 'windmill_flow_chat_inputs_'

	// State for additional inputs modal
	let showInputsModal = $state(false)
	let additionalInputsValues = $state<Record<string, any> | undefined>(
		loadInputsFromStorage() ?? undefined
	)

	function getStorageKey(): string {
		return `${STORAGE_KEY_PREFIX}${path}`
	}

	function loadInputsFromStorage(): Record<string, any> | null {
		try {
			const stored = localStorage.getItem(getStorageKey())
			return stored ? JSON.parse(stored) : null
		} catch (e) {
			console.error('Failed to load inputs from localStorage:', e)
			return null
		}
	}

	function saveInputsToStorage(values: Record<string, any>) {
		try {
			localStorage.setItem(getStorageKey(), JSON.stringify(values))
		} catch (e) {
			console.error('Failed to save inputs to localStorage:', e)
		}
	}

	function handleModalConfirm() {
		saveInputsToStorage(additionalInputsValues ?? {})
		showInputsModal = false
	}

	function openInputsModal() {
		const stored = loadInputsFromStorage()
		if (stored) additionalInputsValues = stored
		showInputsModal = true
	}

	const hasMissingRequired = $derived.by(() => {
		if (!modalSchema?.required?.length) return false
		const values = additionalInputsValues ?? {}
		return modalSchema.required.some((field: string) => isEmptyAgentChatInputValue(values[field]))
	})

	// The host follows the chat it was built on for the life of this component: FlowChat
	// remounts the interface under `{#key chat}`, so a later value of the prop never reaches it.
	const chatHost = new FlowChatViewHost(
		untrack(() => chat),
		{
			additionalInputs: () =>
				additionalInputsSchema ? (loadInputsFromStorage() ?? additionalInputsValues) : undefined,
			attachmentsTarget: () => attachmentsTarget,
			attachmentsUnavailable: () =>
				workspaceStorage.current
					? undefined
					: 'This workspace has no object storage, so files cannot be attached.',
			workspace: () => workspace,
			sendDisabled: () => deploymentInProgress
		}
	)
	setChatViewHost(chatHost)
	onDestroy(() => chatHost.dispose())

	// Older pages load when the reader reaches the top; the viewport stays where it was.
	let scrollElement = $state<HTMLDivElement | undefined>(undefined)
	let loadingOlder = false
	async function handleTranscriptScroll() {
		const state = chatHost.state
		if (!scrollElement || !state.hasMoreMessages || state.loadingMessages || loadingOlder) return
		if (scrollElement.scrollTop > 10) return
		loadingOlder = true
		const previousHeight = scrollElement.scrollHeight
		try {
			await chat.loadOlderMessages()
			await tick()
			scrollElement.scrollTop = scrollElement.scrollHeight - previousHeight
		} finally {
			loadingOlder = false
		}
	}
</script>

<!-- Additional Inputs Modal -->
{#if modalSchema}
	<Modal title="Configure inputs" bind:open={showInputsModal}>
		<SchemaForm
			schema={modalSchema}
			bind:args={additionalInputsValues}
			helperScript={dynamicInputHelperScript}
			{workspace}
		/>
		{#snippet actions()}
			<Button onClick={handleModalConfirm} variant="accent">Save</Button>
		{/snippet}
	</Modal>
{/if}

{#snippet emptyHint()}
	<div class="flex-1 text-center text-tertiary flex items-center justify-center flex-col">
		{#if chatHost.state.loadingMessages}
			<Loader2 size={32} class="animate-spin" />
		{:else}
			<MessageCircle size={48} class="mx-auto mb-4 opacity-50" />
			<p class="text-lg font-medium">Start a conversation</p>
			<p class="text-sm">Send a message to run the flow and see the results</p>
			{#if !emptyString(description)}
				<div class="mt-6 pt-4 border-t max-w-md text-left text-xs text-tertiary">
					<GfmMarkdown md={description ?? ''} noPadding prose="sm" />
				</div>
			{/if}
		{/if}
	</div>
{/snippet}

{#snippet footerSettings()}
	{#if modalSchema}
		<div class="relative">
			<Button
				unifiedSize="2xs"
				variant="subtle"
				startIcon={{ icon: Settings2 }}
				btnClasses="text-secondary font-normal"
				title="Configure the flow inputs sent with each message"
				onClick={openInputsModal}
			>
				Inputs
			</Button>
			{#if hasMissingRequired}
				<span class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-yellow-500 rounded-full"></span>
			{/if}
		</div>
	{/if}
{/snippet}

<!-- The transcript scroller fills its flex row, which needs a height to resolve
     against. Not every host gives one (the editor's Test-flow panel stacks the
     chat above the job result in an auto-height column), so claim one: enough to
     scroll in once there are messages, and before that enough for the empty-state
     prompt and the composer. -->
<div
	class="flex flex-col h-full flex-1 min-w-0"
	class:min-h-96={chatHost.displayMessages.length > 0}
	class:min-h-64={chatHost.displayMessages.length === 0}
>
	<AIChatDisplay
		messages={chatHost.displayMessages}
		bind:scrollElement
		onTranscriptScroll={handleTranscriptScroll}
		pastChats={[]}
		diffMode={false}
		selectedContext={[]}
		availableContext={[]}
		hideHeader
		hideModeSelector
		{wideLayout}
		{emptyHint}
		footerSettings={modalSchema ? footerSettings : undefined}
		placeholder="Send a message to run the flow"
		disabled={deploymentInProgress}
		disabledMessage={deploymentInProgress ? 'Deployment in progress' : ''}
		loadPastChat={() => {}}
		deletePastChat={() => {}}
		saveAndClear={() => {}}
	/>
</div>
