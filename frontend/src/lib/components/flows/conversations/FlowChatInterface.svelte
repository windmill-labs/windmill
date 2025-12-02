<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { MessageCircle, Loader2 } from 'lucide-svelte'
	import ChatMessage from '$lib/components/chat/ChatMessage.svelte'
	import ChatInput from '$lib/components/chat/ChatInput.svelte'
	import { FlowChatManager } from './FlowChatManager.svelte'

	interface Props {
		manager: FlowChatManager
		deploymentInProgress?: boolean
	}

	let { manager, deploymentInProgress = false }: Props = $props()
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
				onSend={() => manager.sendMessage()}
				onKeydown={manager.handleKeyDown}
				showCancelButton={manager.isWaitingForResponse || manager.isLoading}
				onCancel={() => manager.cancelCurrentJob()}
				sendTitle={deploymentInProgress ? 'Deployment in progress' : 'Send message (Enter)'}
			/>
		</div>
	</div>
</div>
