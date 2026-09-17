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
		PER_TURN_AGENT_CHAT_INPUT_KEY,
		resolveAgentChatInputs
	} from './agentAttachmentInput'
	import { deepEqual } from 'fast-equals'
	import FlowChatModelSettings from './FlowChatModelSettings.svelte'
	import {
		agentModelGap,
		composerOwnedInputs,
		resolveAgentModelWiring,
		showsModelButton,
		withoutRejectedEffort
	} from './agentChatInputs'

	interface Props {
		chat: Chat
		deploymentInProgress?: boolean
		additionalInputsSchema?: Record<string, any>
		/** The flow's modules, read for the AI agent inputs the composer drives: the provider wiring
		 * and the attachments input. */
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
		flowModules,
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

	// The model gets its own button, shaped like the copilot's model settings, driven by
	// whichever provider fields the flow exposes. Attachments are the paperclip's; every other
	// flow input is asked for in the Configure-inputs modal.
	const modelWiring = $derived(resolveAgentModelWiring(flowModules))
	// An agent with nothing to call cannot answer, and the composer cannot fix it, so the
	// chat says what to go and do instead of offering controls that write nowhere.
	const modelGap = $derived(agentModelGap(modelWiring))
	const showModelButton = $derived(showsModelButton(modelWiring))

	// LocalStorage helpers
	const STORAGE_KEY_PREFIX = 'windmill_flow_chat_inputs_'

	let showInputsModal = $state(false)
	// Conversation settings, persisted per flow: what the reader chose, and nothing else.
	let inputValues = $state<Record<string, any>>(loadInputsFromStorage() ?? {})
	let modalDraft = $state<Record<string, any>>({})

	/** What the flow's own form would open on. */
	function schemaDefaults(schema: Record<string, any> | undefined): Record<string, any> {
		const properties: Record<string, any> = schema?.properties ?? {}
		return Object.fromEntries(
			Object.entries(properties)
				.filter(([, property]) => property?.default !== undefined)
				.map(([name, property]) => [name, property.default])
		)
	}

	// Derived rather than seeded into `inputValues`: the schema arrives with the flow, which
	// on the deployed page is after this mounts, and only what the reader actually chose
	// belongs in storage. A stored value wins over the default, including a deliberate empty.
	const effectiveInputs = $derived({
		...schemaDefaults(additionalInputsSchema),
		...inputValues
	})

	// What the run actually gets. The composer's own controls keep themselves consistent as
	// they are used; this is where a pair that was never chosen through them — a stored
	// value, an author's default — is made safe before it reaches the provider.
	const runInputs = $derived(withoutRejectedEffort(modelWiring, effectiveInputs))

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
		// The modal opens on `effectiveInputs`, so its draft carries a value for every
		// defaulted input whether or not the reader touched one. Storing those would pin
		// today's defaults for good — `effectiveInputs` gives a stored value precedence, so
		// a later change to the flow's schema would never reach this reader again.
		const defaults = schemaDefaults(additionalInputsSchema)
		const kept = Object.fromEntries(
			Object.entries({ ...inputValues, ...modalDraft }).filter(
				([name, value]) => !deepEqual(value, defaults[name])
			)
		)
		inputValues = kept
		saveInputsToStorage(inputValues)
		showInputsModal = false
	}

	function openInputsModal() {
		modalDraft = { ...effectiveInputs, ...(loadInputsFromStorage() ?? inputValues) }
		showInputsModal = true
	}

	// The host follows the chat it was built on for the life of this component: FlowChat
	// remounts the interface under `{#key chat}`, so a later value of the prop never reaches it.
	const chatHost = new FlowChatViewHost(
		untrack(() => chat),
		{
			additionalInputs: () => (additionalInputsSchema ? { ...runInputs } : undefined),
			attachmentsTarget: () => attachmentsTarget,
			attachmentsUnavailable: () =>
				workspaceStorage.current
					? undefined
					: 'This workspace has no object storage, so files cannot be attached.',
			workspace: () => workspace,
			sendDisabled: () => deploymentInProgress || !!modelGap
		}
	)
	setChatViewHost(chatHost)
	onDestroy(() => chatHost.dispose())

	// What the Configure-inputs modal asks for: every flow input the composer does not
	// edit itself.
	const modalSchema = $derived.by(() => {
		if (!additionalInputsSchema) return undefined
		const promoted = new Set(composerOwnedInputs(modelWiring, attachmentsTarget))
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

	const modalMissingRequired = $derived.by(() => {
		if (!modalSchema?.required?.length) return false
		return modalSchema.required.some((field: string) => {
			const value = effectiveInputs[field]
			return value === undefined || value === '' || value === null
		})
	})

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

{#if modalSchema}
	<Modal title="Configure inputs" bind:open={showInputsModal}>
		<SchemaForm
			schema={modalSchema}
			bind:args={modalDraft}
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
			{#if modalMissingRequired}
				<span class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-yellow-500 rounded-full"></span>
			{/if}
		</div>
	{/if}
	{#if modelWiring && showModelButton}
		<!-- `runInputs`, not `effectiveInputs`: a stored effort the model rejects is dropped
		     before the run, and the button must not name one the run will not send. -->
		<FlowChatModelSettings
			wiring={modelWiring}
			values={runInputs}
			setValue={setInputValue}
			{workspace}
		/>
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
		footerSettings={modalSchema || showModelButton ? footerSettings : undefined}
		placeholder="Send a message to run the flow"
		disabled={deploymentInProgress || !!modelGap}
		disabledMessage={deploymentInProgress ? 'Deployment in progress' : (modelGap ?? '')}
		loadPastChat={() => {}}
		deletePastChat={() => {}}
		saveAndClear={() => {}}
	/>
</div>
