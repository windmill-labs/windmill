<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { MessageCircle, Loader2, Settings2 } from 'lucide-svelte'
	import ChatMessage from '$lib/components/chat/ChatMessage.svelte'
	import ChatInput from '$lib/components/chat/ChatInput.svelte'
	import { FlowChatManager } from './FlowChatManager.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { untrack } from 'svelte'

	interface Props {
		manager: FlowChatManager
		deploymentInProgress?: boolean
		additionalInputsSchema?: Record<string, any>
		path?: string
	}

	let { manager, deploymentInProgress = false, additionalInputsSchema, path }: Props = $props()

	// State for additional inputs modal
	let showInputsModal = $state(false)
	let additionalInputsValues = $state<Record<string, any>>({})

	// LocalStorage helpers
	const STORAGE_KEY_PREFIX = 'windmill_flow_chat_inputs_'

	function getStorageKey(conversationId: string | undefined): string {
		return `${STORAGE_KEY_PREFIX}${path ?? 'unknown'}_${conversationId ?? 'new'}`
	}

	function loadInputsFromStorage(conversationId: string | undefined): Record<string, any> | null {
		try {
			const stored = localStorage.getItem(getStorageKey(conversationId))
			return stored ? JSON.parse(stored) : null
		} catch {
			return null
		}
	}

	function saveInputsToStorage(conversationId: string | undefined, values: Record<string, any>) {
		try {
			localStorage.setItem(getStorageKey(conversationId), JSON.stringify(values))
		} catch (e) {
			console.error('Failed to save inputs to localStorage:', e)
		}
	}

	function migrateNewToConversationId(newConversationId: string) {
		const newKey = getStorageKey(undefined) // 'new' key
		try {
			const stored = localStorage.getItem(newKey)
			if (stored) {
				localStorage.setItem(getStorageKey(newConversationId), stored)
				localStorage.removeItem(newKey)
			}
		} catch (e) {
			console.error('Failed to migrate localStorage:', e)
		}
	}

	// Show modal on conversation focus/selection
	let lastCheckedConversationId: string | undefined = undefined
	$effect(() => {
		const conversationId = manager.selectedConversationId

		// Only trigger when conversation actually changes
		if (conversationId === lastCheckedConversationId) return

		untrack(() => {
			lastCheckedConversationId = conversationId

			if (additionalInputsSchema) {
				const stored = loadInputsFromStorage(conversationId)
				if (stored) {
					additionalInputsValues = stored
				} else {
					// No values stored, show modal
					additionalInputsValues = {}
					showInputsModal = true
				}
			}
		})
	})

	function handleModalConfirm() {
		const conversationId = manager.selectedConversationId
		saveInputsToStorage(conversationId, additionalInputsValues)
		showInputsModal = false
	}

	function handleModalCancel() {
		showInputsModal = false
	}

	function handleSendMessage() {
		const conversationId = manager.selectedConversationId
		const inputs = additionalInputsSchema
			? loadInputsFromStorage(conversationId) ?? additionalInputsValues
			: undefined
		manager.sendMessage(inputs)
	}

	function openInputsModal() {
		const stored = loadInputsFromStorage(manager.selectedConversationId)
		if (stored) additionalInputsValues = stored
		showInputsModal = true
	}

	// Called when a new conversation is created to migrate localStorage
	function onConversationCreated(newConversationId: string) {
		migrateNewToConversationId(newConversationId)
	}

	// Set the callback on the manager when mounted
	$effect(() => {
		if (additionalInputsSchema) {
			manager.setOnConversationCreated(onConversationCreated)
		}
	})
</script>

<!-- Additional Inputs Modal -->
{#if additionalInputsSchema}
	<Modal title="Configure Additional Inputs" bind:open={showInputsModal}>
		<div class="p-4">
			<SchemaForm schema={additionalInputsSchema} bind:args={additionalInputsValues} />
		</div>
		{#snippet actions()}
			<Button variant="border" on:click={handleModalCancel}>Cancel</Button>
			<Button on:click={handleModalConfirm}>Save</Button>
		{/snippet}
	</Modal>
{/if}

<div class="flex flex-col h-full flex-1 min-w-0">
	<!-- Header with settings button -->
	{#if additionalInputsSchema}
		<div class="flex items-center justify-end px-4 py-2 border-b">
			<Button
				iconOnly
				size="xs"
				variant="border"
				startIcon={{ icon: Settings2 }}
				title="Edit additional inputs"
				on:click={openInputsModal}
			/>
		</div>
	{/if}

	<!-- Messages Container -->
	<div
		bind:this={manager.messagesContainer}
		class="flex-1 min-h-0 overflow-y-auto p-4 bg-background"
		onscroll={manager.handleScroll}
	>
		{#if deploymentInProgress}
			<Alert type="warning" title="Deployment in progress" size="xs" />
		{/if}
		{#if manager.isLoadingMessages}
			<div class="flex items-center justify-center h-full">
				<Loader2 size={32} class="animate-spin" />
			</div>
		{:else if manager.messages.length === 0}
			<div class="text-center text-tertiary flex items-center justify-center flex-col h-full">
				<MessageCircle size={48} class="mx-auto mb-4 opacity-50" />
				<p class="text-lg font-medium">Start a conversation</p>
				<p class="text-sm">Send a message to run the flow and see the results</p>
			</div>
		{:else}
			<div class="w-full space-y-4 xl:max-w-7xl mx-auto">
				{#each manager.messages as message (message.id)}
					<ChatMessage
						role={message.message_type}
						content={message.content}
						loading={message.loading}
						success={message.success}
						stepName={message.step_name}
					/>
				{/each}
				{#if manager.isWaitingForResponse}
					<div class="flex items-center gap-2 text-tertiary">
						<Loader2 size={16} class="animate-spin" />
						<span class="text-sm">Processing...</span>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Chat Input -->
	<div class="flex flex-row justify-center py-2 xl:max-w-7xl mx-auto w-full">
		<div class="w-full" class:opacity-50={deploymentInProgress}>
			<ChatInput
				bind:value={manager.inputMessage}
				bind:bindTextarea={manager.inputElement}
				disabled={manager.isLoading || deploymentInProgress}
				onSend={handleSendMessage}
				onKeydown={(e) => {
					if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
						e.preventDefault()
						handleSendMessage()
					}
				}}
				showCancelButton={manager.isWaitingForResponse || manager.isLoading}
				onCancel={() => manager.cancelCurrentJob()}
				sendTitle={deploymentInProgress ? 'Deployment in progress' : 'Send message (Enter)'}
			/>
		</div>
	</div>
</div>
