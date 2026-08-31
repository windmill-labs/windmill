<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Loader2, MessageCircle, Settings2 } from 'lucide-svelte'
	import { FlowChatManager } from './FlowChatManager.svelte'
	import { FlowChatViewHost } from './flowChatViewHost.svelte'
	import AIChatDisplay from '$lib/components/copilot/chat/AIChatDisplay.svelte'
	import { setChatViewHost } from '$lib/components/copilot/chat/chatViewHost'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { type DynamicInput } from '$lib/utils'

	interface Props {
		manager: FlowChatManager
		deploymentInProgress?: boolean
		additionalInputsSchema?: Record<string, any>
		path: string
		wideLayout?: boolean
	}

	let {
		manager,
		deploymentInProgress = false,
		additionalInputsSchema,
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

	const chatHost = new FlowChatViewHost(manager, () =>
		additionalInputsSchema ? (loadInputsFromStorage() ?? additionalInputsValues) : undefined
	)
	setChatViewHost(chatHost)

	// A message typed mid-run is held by the host; send it once the run settles.
	$effect(() => {
		if (!chatHost.loading && chatHost.queuedMessage) {
			chatHost.flushQueuedMessage()
		}
	})

	const hasMissingRequired = $derived.by(() => {
		if (!additionalInputsSchema?.required?.length) return false
		const values = additionalInputsValues ?? {}
		return additionalInputsSchema.required.some(
			(field: string) =>
				values[field] === undefined || values[field] === '' || values[field] === null
		)
	})
</script>

<!-- Additional Inputs Modal -->
{#if additionalInputsSchema}
	<Modal title="Configure inputs" bind:open={showInputsModal}>
		<SchemaForm
			schema={additionalInputsSchema}
			bind:args={additionalInputsValues}
			helperScript={dynamicInputHelperScript}
			workspace={manager.operatingWorkspace?.()}
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

{#snippet inputPreface()}
	{#if additionalInputsSchema}
		<div class="flex items-center justify-end w-full mb-1">
			<div class="relative">
				<Button
					unifiedSize="xs"
					variant="default"
					startIcon={{ icon: Settings2 }}
					title="Inputs"
					onClick={openInputsModal}
				>
					Inputs
				</Button>
				{#if hasMissingRequired}
					<span class="absolute -top-1 -right-1 w-2 h-2 bg-yellow-500 rounded-full"></span>
				{/if}
			</div>
		</div>
	{/if}
{/snippet}

<!-- The transcript scroller fills its flex row, which needs a height to resolve
     against. Not every host gives one (the editor's Test-flow panel stacks the
     chat above the job result in an auto-height column), so once there are
     messages to scroll, claim one. -->
<div
	class="flex flex-col h-full flex-1 min-w-0"
	class:min-h-96={chatHost.displayMessages.length > 0}
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
		{inputPreface}
		placeholder="Send a message to run the flow"
		disabled={deploymentInProgress}
		disabledMessage="Deployment in progress"
		loadPastChat={() => {}}
		deletePastChat={() => {}}
		saveAndClear={() => {}}
	/>
</div>
