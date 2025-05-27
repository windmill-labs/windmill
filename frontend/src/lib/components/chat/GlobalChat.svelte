<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Send, X, Loader2, MessageCircle } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	export let open = false

	const dispatch = createEventDispatcher<{
		close: null
	}>()

	let message = ''
	let messages: Array<{ role: 'user' | 'assistant'; content: string; timestamp: Date }> = []
	let loading = writable(false)
	let chatContainer: HTMLDivElement

	// Placeholder chat history for demo
	let chatHistory = [
		{
			role: 'assistant' as const,
			content: 'Hello! I\'m your global assistant. How can I help you today?',
			timestamp: new Date(Date.now() - 60000)
		}
	]

	$: messages = [...chatHistory, ...messages]

	function scrollToBottom() {
		if (chatContainer) {
			setTimeout(() => {
				chatContainer.scrollTop = chatContainer.scrollHeight
			}, 10)
		}
	}

	async function sendMessage() {
		if (!message.trim() || $loading) return

		const userMessage = message.trim()
		message = ''

		// Add user message
		messages = [...messages, {
			role: 'user',
			content: userMessage,
			timestamp: new Date()
		}]

		scrollToBottom()

		// Simulate API call with placeholder response
		loading.set(true)
		
		try {
			// Simulate processing delay
			await new Promise(resolve => setTimeout(resolve, 1500))
			
			// Add placeholder assistant response
			const responses = [
				"I understand you're asking about: " + userMessage + ". This is a placeholder response for the global chat feature.",
				"That's an interesting question! The global chat functionality is currently being developed with placeholder responses.",
				"Thanks for your message: \"" + userMessage + "\". In the full implementation, this would connect to the AI system.",
				"I see you mentioned: " + userMessage + ". This global chat drawer is now functional with placeholder logic as requested."
			]
			
			const response = responses[Math.floor(Math.random() * responses.length)]
			
			messages = [...messages, {
				role: 'assistant',
				content: response,
				timestamp: new Date()
			}]

			scrollToBottom()
		} catch (error) {
			sendUserToast('Error sending message', true)
		} finally {
			loading.set(false)
		}
	}

	function handleKeyPress(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault()
			sendMessage()
		}
	}

	function formatTime(date: Date) {
		return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
	}

	function clearChat() {
		messages = []
		chatHistory = [
			{
				role: 'assistant' as const,
				content: 'Chat cleared! How can I help you now?',
				timestamp: new Date()
			}
		]
		scrollToBottom()
	}
</script>

<div class="flex flex-col h-full bg-surface">
	<!-- Header -->
	<div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-600">
		<div class="flex items-center gap-2">
			<MessageCircle size={18} class="text-primary" />
			<h2 class="text-lg font-semibold">Global Chat</h2>
		</div>
		<div class="flex items-center gap-2">
			<Button
				size="sm"
				variant="border"
				color="light"
				on:click={clearChat}
			>
				Clear
			</Button>
			<Button
				size="sm"
				variant="border"
				color="light"
				iconOnly
				startIcon={{ icon: X }}
				on:click={() => dispatch('close')}
			/>
		</div>
	</div>

	<!-- Chat Messages -->
	<div 
		bind:this={chatContainer}
		class="flex-1 overflow-y-auto p-4 space-y-4"
	>
		{#each messages as msg (msg.timestamp.getTime())}
			<div class={twMerge(
				"flex flex-col",
				msg.role === 'user' ? "items-end" : "items-start"
			)}>
				<div class={twMerge(
					"max-w-[80%] p-3 rounded-lg text-sm",
					msg.role === 'user' 
						? "bg-blue-500 text-white rounded-br-sm" 
						: "bg-gray-100 dark:bg-gray-700 text-primary rounded-bl-sm"
				)}>
					<p class="whitespace-pre-wrap">{msg.content}</p>
				</div>
				<span class="text-xs text-secondary mt-1">
					{formatTime(msg.timestamp)}
				</span>
			</div>
		{/each}
		
		{#if $loading}
			<div class="flex items-start">
				<div class="bg-gray-100 dark:bg-gray-700 p-3 rounded-lg rounded-bl-sm">
					<div class="flex items-center gap-2 text-secondary">
						<Loader2 size={16} class="animate-spin" />
						<span class="text-sm">Thinking...</span>
					</div>
				</div>
			</div>
		{/if}
	</div>

	<!-- Input Area -->
	<div class="p-4 border-t border-gray-200 dark:border-gray-600">
		<div class="flex gap-2">
			<textarea
				bind:value={message}
				on:keydown={handleKeyPress}
				placeholder="Type your message... (Enter to send, Shift+Enter for new line)"
				class="flex-1 resize-none border border-gray-300 dark:border-gray-600 rounded-lg p-3 text-sm bg-surface text-primary focus:outline-none focus:ring-2 focus:ring-blue-500 min-h-[44px] max-h-32"
				rows="1"
				disabled={$loading}
			></textarea>
			<Button
				size="md"
				disabled={!message.trim() || $loading}
				iconOnly
				startIcon={{ icon: Send }}
				on:click={sendMessage}
			/>
		</div>
		<p class="text-xs text-secondary mt-2">
			Global chat with placeholder functionality - ready for AI integration
		</p>
	</div>
</div>