<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Send, Loader2 } from 'lucide-svelte'
	import { chatRequest, prepareSystemMessage } from './core'
	import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'

	let chatHistory = [
		{
			role: 'assistant',
			content: "Hello! I'm your global assistant. How can I help you today?"
		}
	] as ChatCompletionMessageParam[]

	let inputValue = $state('')
	let isSubmitting = $state(false)
	let currentReply = $state('')
	let messages = $state(chatHistory)

	let abortController = new AbortController()
	let chatContainer: HTMLDivElement

	async function handleSubmit() {
		if (!inputValue.trim()) return

		isSubmitting = true
		currentReply = ''

		const userMessage = inputValue
		const systemMessage = prepareSystemMessage()

		// Add user message to chat
		messages = [...messages, { role: 'user', content: userMessage }]

		// Create message array for API request
		const apiMessages = [systemMessage, ...messages]

		await chatRequest(apiMessages, abortController, (token) => {
			currentReply = currentReply + token
		})

		// Add assistant's response to chat
		messages = [...messages, { role: 'assistant', content: currentReply }]

		// Reset the input field
		inputValue = ''
		isSubmitting = false
	}
</script>

<div class="flex flex-col h-full bg-surface z-10">
	<!-- Chat Messages -->
	<div bind:this={chatContainer} class="flex-1 overflow-y-auto p-4 space-y-4 z-10">
		{#each messages as msg}
			<div class={twMerge('flex flex-col', msg.role === 'user' ? 'items-end' : 'items-start')}>
				<div
					class={twMerge(
						'max-w-[80%] p-3 rounded-lg text-sm',
						msg.role === 'user'
							? 'bg-blue-500 text-white rounded-br-sm'
							: 'bg-gray-100 dark:bg-gray-700 text-primary rounded-bl-sm'
					)}
				>
					<p class="whitespace-pre-wrap">{msg.content}</p>
				</div>
			</div>
		{/each}

		{#if isSubmitting}
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
				bind:value={inputValue}
				onkeydown={(e) => {
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault()
						handleSubmit()
					}
				}}
				placeholder="Type your message..."
				class="flex-1 resize-none border border-gray-300 dark:border-gray-600 rounded-lg p-3 text-sm bg-surface text-primary focus:outline-none focus:ring-2 focus:ring-blue-500 min-h-[44px] max-h-32 z-10"
				rows="1"
				disabled={isSubmitting}
			></textarea>
			<Button
				size="md"
				disabled={!inputValue.trim() || isSubmitting}
				iconOnly
				startIcon={{ icon: Send }}
				on:click={handleSubmit}
			/>
		</div>
	</div>
</div>
