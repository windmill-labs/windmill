<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Loader2, MessageCircle, SlidersHorizontal } from 'lucide-svelte'
	import { FlowChatManager } from './FlowChatManager.svelte'
	import { FlowChatViewHost } from './flowChatViewHost.svelte'
	import AIChatDisplay from '$lib/components/copilot/chat/AIChatDisplay.svelte'
	import { setChatViewHost } from '$lib/components/copilot/chat/chatViewHost'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { type DynamicInput } from '$lib/utils'
	import { CancelError, WorkspaceService, type FlowModule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'
	import FlowChatModelSettings from './FlowChatModelSettings.svelte'
	import {
		agentModelGap,
		agentModelWiringInputs,
		isEmptyAgentChatInputValue,
		PER_TURN_AGENT_CHAT_INPUT_KEY,
		resolveAgentChatInputs,
		resolveAgentModelWiring
	} from './agentChatInputs'

	interface Props {
		manager: FlowChatManager
		deploymentInProgress?: boolean
		additionalInputsSchema?: Record<string, any>
		/** The flow's modules, used to find which inputs an AI agent step reads directly. */
		flowModules?: FlowModule[]
		path: string
		wideLayout?: boolean
	}

	let {
		manager,
		deploymentInProgress = false,
		additionalInputsSchema,
		flowModules,
		path,
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

	// Inputs an AI agent step reads straight out of the flow input get a composer chip
	// instead of a modal field; the rest stay in the modal.
	const agentChatInputs = $derived(resolveAgentChatInputs(flowModules, additionalInputsSchema))
	// The composer's attachments feed this input; it never appears as a chip or a
	// modal field, because the paperclip is its editor.
	const attachmentsInput = $derived(
		agentChatInputs.find((input) => input.key === PER_TURN_AGENT_CHAT_INPUT_KEY)
	)
	const attachmentsTarget = $derived(
		attachmentsInput
			? { name: attachmentsInput.name, multiple: attachmentsInput.property?.type === 'array' }
			: undefined
	)

	const chatWorkspace = $derived(manager.operatingWorkspace?.() ?? $workspaceStore)

	// Uploading needs the workspace's object storage; without one the `+` stays hidden
	// rather than failing on drop. Assumed absent until this workspace's answer lands.
	const settings = resource(
		() => chatWorkspace,
		async (ws, _previous, { onCleanup }) => {
			if (!ws) return undefined
			const req = WorkspaceService.getPublicSettings({ workspace: ws })
			onCleanup(() => req.cancel())
			try {
				return { ws, settings: await req }
			} catch (err) {
				if (!(err instanceof CancelError)) {
					console.error('Failed to fetch workspace settings:', err)
				}
				return undefined
			}
		}
	)
	const s3StorageConfigured = $derived.by(() => {
		const loaded = settings.current
		if (!loaded || loaded.ws !== chatWorkspace) return false
		return loaded.settings.large_file_storage?.s3_resource_path !== undefined
	})
	// The model gets its own button, shaped like the copilot's model settings, driven by
	// whichever provider fields the flow exposes. Attachments are the paperclip's. Nothing
	// else is promoted, so every other flow input is asked for in the Configure-inputs modal.
	const modelWiring = $derived(resolveAgentModelWiring(flowModules))
	// An agent with nothing to call cannot answer, and the composer cannot fix it, so the
	// chat says what to go and do instead of offering controls that write nowhere.
	const modelGap = $derived(agentModelGap(modelWiring))

	const modalSchema = $derived.by(() => {
		if (!additionalInputsSchema) return undefined
		const promoted = new Set([
			...agentChatInputs.map((input) => input.name),
			...agentModelWiringInputs(modelWiring)
		])
		const properties = Object.fromEntries(
			Object.entries(additionalInputsSchema.properties ?? {}).filter(([key]) => !promoted.has(key))
		)
		if (Object.keys(properties).length === 0) return undefined
		const required: string[] = Array.isArray(additionalInputsSchema.required)
			? additionalInputsSchema.required
			: []
		return {
			...additionalInputsSchema,
			properties,
			required: required.filter((key) => !promoted.has(key))
		}
	})

	// LocalStorage helpers
	const STORAGE_KEY_PREFIX = 'windmill_flow_chat_inputs_'

	let showInputsModal = $state(false)
	// Conversation settings, persisted per flow. Attachments are absent by construction:
	// they ride the composer's own draft and are cleared with it on send.
	let inputValues = $state<Record<string, any>>(loadInputsFromStorage() ?? {})
	let modalDraft = $state<Record<string, any>>({})

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

	function setInputValue(name: string, value: any) {
		inputValues = { ...inputValues, [name]: value }
		saveInputsToStorage(inputValues)
	}

	function handleModalConfirm() {
		inputValues = { ...inputValues, ...modalDraft }
		saveInputsToStorage(inputValues)
		showInputsModal = false
	}

	function openInputsModal() {
		modalDraft = { ...(loadInputsFromStorage() ?? inputValues) }
		showInputsModal = true
	}

	const chatHost = new FlowChatViewHost(manager, {
		additionalInputs: () => (additionalInputsSchema ? { ...inputValues } : undefined),
		attachmentsTarget: () => attachmentsTarget,
		workspace: () => chatWorkspace,
		canAttach: () => s3StorageConfigured
	})
	setChatViewHost(chatHost)

	// A message typed mid-run is held by the host; send it once the run settles.
	$effect(() => {
		if (!chatHost.loading && chatHost.queuedMessage) {
			chatHost.flushQueuedMessage()
		}
	})

	const modalMissingRequired = $derived.by(() => {
		if (!modalSchema?.required?.length) return false
		return modalSchema.required.some((field: string) =>
			isEmptyAgentChatInputValue(inputValues[field])
		)
	})
</script>

{#if modalSchema}
	<Modal title="Configure inputs" bind:open={showInputsModal}>
		<SchemaForm
			schema={modalSchema}
			bind:args={modalDraft}
			helperScript={dynamicInputHelperScript}
			workspace={chatWorkspace}
		/>
		{#snippet actions()}
			<Button onClick={handleModalConfirm} variant="accent">Save</Button>
		{/snippet}
	</Modal>
{/if}

{#snippet emptyHint()}
	<div class="flex-1 text-center text-tertiary flex items-center justify-center flex-col">
		{#if manager.isLoadingMessages}
			<Loader2 size={32} class="animate-spin" />
		{:else}
			<MessageCircle size={48} class="mx-auto mb-4 opacity-50" />
			<p class="text-lg font-medium">Start a conversation</p>
			<p class="text-sm">Send a message to run the flow and see the results</p>
		{/if}
	</div>
{/snippet}

{#snippet footerSettings()}
	{#if modalSchema}
		<div class="relative">
			<Button
				unifiedSize="2xs"
				variant="subtle"
				startIcon={{ icon: SlidersHorizontal }}
				btnClasses="text-secondary font-normal"
				title="Configure the flow inputs sent with each message"
				onClick={openInputsModal}
			>
				Inputs
			</Button>
			{#if modalMissingRequired}
				<span class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-yellow-500 rounded-full"></span>
			{/if}
		</div>
	{/if}
	{#if modelWiring}
		<FlowChatModelSettings
			wiring={modelWiring}
			values={inputValues}
			setValue={setInputValue}
			workspace={chatWorkspace}
		/>
	{/if}
{/snippet}

<!-- The transcript scroller fills its flex row, which needs a height to resolve
     against. Not every host gives one (the editor's Test-flow panel stacks the
     chat above the job result in an auto-height column), so claim one: enough to
     scroll in once there are messages, and before that enough for the empty-state
     prompt and the composer, which otherwise crowd the panel they collapse it to. -->
<div
	class="flex flex-col h-full flex-1 min-w-0"
	class:min-h-96={chatHost.displayMessages.length > 0}
	class:min-h-64={chatHost.displayMessages.length === 0}
>
	<AIChatDisplay
		messages={chatHost.displayMessages}
		bind:scrollElement={manager.messagesContainer}
		onTranscriptScroll={manager.handleScroll}
		pastChats={[]}
		diffMode={false}
		selectedContext={[]}
		availableContext={[]}
		hideHeader
		hideModeSelector
		{wideLayout}
		{emptyHint}
		footerSettings={modalSchema || modelWiring ? footerSettings : undefined}
		placeholder="Send a message to run the flow"
		disabled={deploymentInProgress || !!modelGap}
		disabledMessage={deploymentInProgress ? 'Deployment in progress' : (modelGap ?? '')}
		loadPastChat={() => {}}
		deletePastChat={() => {}}
		saveAndClear={() => {}}
	/>
</div>
