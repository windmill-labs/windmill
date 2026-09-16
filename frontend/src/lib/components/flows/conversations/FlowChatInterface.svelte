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
	import { chatFlowKey } from './flowChatProps'

	interface Props {
		chat: Chat
		deploymentInProgress?: boolean
		additionalInputsSchema?: Record<string, any>
		path: string
		/** What the stored inputs are filed under when the path is not steady (see FlowChat). */
		identity?: string
		workspace?: string
		/** The flow's description, shown under the empty transcript's prompt. */
		description?: string
		wideLayout?: boolean
	}

	let {
		chat,
		deploymentInProgress = false,
		additionalInputsSchema,
		path,
		identity = undefined,
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

	// LocalStorage helpers
	const STORAGE_KEY_PREFIX = 'windmill_flow_chat_inputs_'

	// State for additional inputs modal
	let showInputsModal = $state(false)
	let additionalInputsValues = $state<Record<string, any> | undefined>(
		loadInputsFromStorage() ?? undefined
	)

	function getStorageKey(): string {
		return `${STORAGE_KEY_PREFIX}${chatFlowKey({ path, identity })}`
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
		if (!additionalInputsSchema?.required?.length) return false
		const values = additionalInputsValues ?? {}
		return additionalInputsSchema.required.some(
			(field: string) =>
				values[field] === undefined || values[field] === '' || values[field] === null
		)
	})

	// The host follows the chat it was built on for the life of this component: FlowChat
	// remounts the interface under `{#key chat}`, so a later value of the prop never reaches it.
	const chatHost = new FlowChatViewHost(
		untrack(() => chat),
		{
			additionalInputs: () =>
				additionalInputsSchema ? (loadInputsFromStorage() ?? additionalInputsValues) : undefined,
			workspace: () => workspace,
			sendDisabled: () => deploymentInProgress,
			inputsSchema: () => additionalInputsSchema
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
{#if additionalInputsSchema}
	<Modal title="Configure inputs" bind:open={showInputsModal}>
		<SchemaForm
			schema={additionalInputsSchema}
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
	{#if additionalInputsSchema}
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
		footerSettings={additionalInputsSchema ? footerSettings : undefined}
		placeholder="Send a message to run the flow"
		disabled={deploymentInProgress}
		disabledMessage={deploymentInProgress ? 'Deployment in progress' : ''}
		loadPastChat={() => {}}
		deletePastChat={() => {}}
		saveAndClear={() => {}}
	/>
</div>
