<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { MessageCircle, Loader2, Settings2 } from 'lucide-svelte'
	import ChatMessage from '$lib/components/chat/ChatMessage.svelte'
	import ChatInput from '$lib/components/chat/ChatInput.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { type DynamicInput } from '$lib/utils'
	import { tick, untrack } from 'svelte'
	import type { Chat, ChatState } from 'windmill-chat'

	interface Props {
		chat: Chat
		chatState: ChatState
		deploymentInProgress?: boolean
		additionalInputsSchema?: Record<string, any>
		path: string
		workspace?: string
	}

	let {
		chat,
		chatState,
		deploymentInProgress = false,
		additionalInputsSchema,
		path,
		workspace = undefined
	}: Props = $props()

	let inputMessage = $state('')
	let inputElement = $state<HTMLTextAreaElement | undefined>(undefined)
	let messagesContainer = $state<HTMLDivElement | undefined>(undefined)
	let loadingOlder = false

	const busy = $derived(chatState.status === 'submitted' || chatState.status === 'streaming')
	// Deriveds notify only when their value changes; `chatState` itself is a new
	// object on every token, and following it would drag a reader who scrolled up
	// back to the end on each one.
	const messageCount = $derived(chatState.messages.length)
	const conversationId = $derived(chatState.conversationId)
	const loadingMessages = $derived(chatState.loadingMessages)

	// Follow the conversation: new messages and a conversation switch scroll to the
	// end, older pages loaded at the top keep the viewport where it was.
	$effect(() => {
		messageCount
		conversationId
		loadingMessages
		untrack(() => {
			if (loadingOlder) return
			tick().then(() => {
				if (messagesContainer) messagesContainer.scrollTop = messagesContainer.scrollHeight
			})
		})
	})

	async function handleScroll() {
		if (
			!messagesContainer ||
			!chatState.hasMoreMessages ||
			chatState.loadingMessages ||
			loadingOlder
		)
			return
		if (messagesContainer.scrollTop > 10) return
		loadingOlder = true
		const previousHeight = messagesContainer.scrollHeight
		try {
			await chat.loadOlderMessages()
			await tick()
			messagesContainer.scrollTop = messagesContainer.scrollHeight - previousHeight
		} finally {
			loadingOlder = false
		}
	}

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

	async function handleSendMessage() {
		const text = inputMessage.trim()
		if (!text || busy || deploymentInProgress) return
		const inputs = additionalInputsSchema
			? (loadInputsFromStorage() ?? additionalInputsValues)
			: undefined
		inputMessage = ''
		// A failure is reported through the chat's `onError` and as a failed message.
		await chat.sendMessage(text, { inputs }).catch(() => {})
		await tick()
		inputElement?.focus()
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

<div class="flex flex-col h-full flex-1 min-w-0">
	<!-- Messages Container -->
	<div
		bind:this={messagesContainer}
		class="flex-1 min-h-0 overflow-y-auto p-4 bg-background"
		onscroll={handleScroll}
	>
		{#if deploymentInProgress}
			<Alert type="warning" title="Deployment in progress" size="xs" />
		{/if}
		{#if chatState.loadingMessages && chatState.messages.length === 0}
			<div class="flex items-center justify-center h-full">
				<Loader2 size={32} class="animate-spin" />
			</div>
		{:else if chatState.messages.length === 0}
			<div class="text-center text-tertiary flex items-center justify-center flex-col h-full">
				<MessageCircle size={48} class="mx-auto mb-4 opacity-50" />
				<p class="text-lg font-medium">Start a conversation</p>
				<p class="text-sm">Send a message to run the flow and see the results</p>
			</div>
		{:else}
			<div class="w-full space-y-4 xl:max-w-7xl mx-auto">
				{#each chatState.messages as message (message.id)}
					<ChatMessage
						role={message.role}
						content={message.content}
						success={message.success}
						stepName={message.stepName}
					/>
				{/each}
				{#if busy}
					<div class="flex items-center gap-2 text-tertiary">
						<Loader2 size={16} class="animate-spin" />
						<span class="text-sm">Processing...</span>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Chat Input -->
	<div class="flex flex-col items-center p-2 xl:max-w-7xl mx-auto w-full gap-2">
		{#if additionalInputsSchema}
			<div class="flex items-center justify-end w-full">
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
		<div class="w-full" class:opacity-50={deploymentInProgress}>
			<ChatInput
				bind:value={inputMessage}
				bind:bindTextarea={inputElement}
				disabled={busy || deploymentInProgress}
				onSend={handleSendMessage}
				onKeydown={(e) => {
					if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
						e.preventDefault()
						handleSendMessage()
					}
				}}
				showCancelButton={busy}
				onCancel={() => chat.stop()}
				sendTitle={deploymentInProgress ? 'Deployment in progress' : 'Send message (Enter)'}
			/>
		</div>
	</div>
</div>
