<script lang="ts">
	import { Button, Alert } from '$lib/components/common'
	import { MessageCircle, Loader2, ArrowUp, Square } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import autosize from '$lib/autosize'
	import FlowChatMessage from './FlowChatMessage.svelte'
	import { createFlowChatManager } from './FlowChatManager.svelte'

	interface Props {
		onRunFlow: (userMessage: string, conversationId: string) => Promise<string | undefined>
		useStreaming?: boolean
		refreshConversations?: () => Promise<void>
		conversationId?: string
		deploymentInProgress?: boolean
		createConversation: (options: { clearMessages?: boolean }) => Promise<string>
		path?: string
	}

	let {
		onRunFlow,
		conversationId,
		refreshConversations,
		deploymentInProgress = false,
		createConversation,
		useStreaming = false,
		path
	}: Props = $props()

	const manager = createFlowChatManager()

	// Initialize manager when component mounts
	$effect(() => {
		if ($workspaceStore) {
			manager.initialize(
				{
					onRunFlow,
					createConversation,
					refreshConversations,
					conversationId,
					useStreaming,
					path
				},
				$workspaceStore
			)
		}

		return () => {
			manager.cleanup()
		}
	})

	// Update conversation ID when it changes
	$effect(() => {
		manager.updateConversationId(conversationId)
	})

	// Public API for parent components
	export function fillInputMessage(message: string) {
		manager.fillInputMessage(message)
	}

	export function focusInput() {
		manager.focusInput()
	}

	export function clearMessages() {
		manager.clearMessages()
	}

	export async function loadConversationMessages(conversationId?: string) {
		await manager.loadConversationMessages(conversationId)
	}
</script>

<div class="flex flex-col h-full flex-1 min-w-0">
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
			<div class="w-full xl:max-w-7xl mx-auto space-y-4">
				{#each manager.messages as message (message.id)}
					<FlowChatMessage {message} />
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
	<div class="p-2 bg-surface">
		<div
			class="flex items-center gap-2 rounded-lg border border-gray-200 dark:border-gray-600 bg-surface"
			class:opacity-50={deploymentInProgress}
		>
			<textarea
				bind:this={manager.inputElement}
				bind:value={manager.inputMessage}
				use:autosize
				onkeydown={manager.handleKeyDown}
				placeholder="Type your message here..."
				class="flex-1 min-h-[24px] max-h-32 resize-none !border-0 !bg-transparent text-sm placeholder-gray-400 !outline-none !ring-0 p-0 !shadow-none focus:!border-0 focus:!outline-none focus:!ring-0 focus:!shadow-none"
				rows={3}
			></textarea>
			<div class="flex-shrink-0 pr-2">
				{#if manager.isWaitingForResponse || manager.isLoading}
					<Button
						color="red"
						size="xs2"
						btnClasses="!rounded-full !p-1.5"
						startIcon={{ icon: Square }}
						on:click={() => manager.cancelCurrentJob()}
						iconOnly
						title="Cancel execution"
					/>
				{:else}
					<Button
						color="blue"
						size="xs2"
						btnClasses="!rounded-full !p-1.5"
						startIcon={{ icon: ArrowUp }}
						disabled={!manager.inputMessage?.trim() || manager.isLoading || deploymentInProgress}
						on:click={() => manager.sendMessage()}
						iconOnly
						title={deploymentInProgress ? 'Deployment in progress' : 'Send message (Enter)'}
					/>
				{/if}
			</div>
		</div>
	</div>
</div>
