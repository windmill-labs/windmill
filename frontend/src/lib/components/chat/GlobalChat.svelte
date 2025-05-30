<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Send, Loader2, RefreshCcw, Plus } from 'lucide-svelte'
	import { chatRequest, prepareSystemMessage } from './core'
	import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
	import { globalChatInitialInput, userStore, copilotInfo } from '$lib/stores'

	const isAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	const hasCopilot = $derived($copilotInfo.enabled)

	const firstMessage = $derived(
		hasCopilot
			? 'Hello! I am your global assistant. How can I help you today?'
			: isAdmin
				? 'Enable Windmill AI in your workspace settings to use this chat'
				: 'Ask an admin to enable Windmill AI in this workspace to use this chat'
	)

	let inputValue = $state('')
	let isSubmitting = $state(false)
	let currentReply = $state('')
	let messages = $state<ChatCompletionMessageParam[]>([])

	// Suggested questions for the user
	const suggestions = [
		'Where can i see my latest runs?',
		'How do i trigger a script with a webhook endpoint?',
		'How can I connect to a database?',
		'How do I schedule a recurring job?'
	]

	// Check if there are any user messages
	const hasUserMessages = $derived(messages.some((msg) => msg.role === 'user'))

	let abortController = new AbortController()

	async function handleSubmit() {
		if (!inputValue.trim()) return

		isSubmitting = true
		currentReply = ''

		const userMessage = inputValue
		const systemMessage = prepareSystemMessage()

		messages = [...messages, { role: 'user', content: userMessage }]

		const apiMessages = [systemMessage, ...messages]

		inputValue = ''

		await chatRequest(apiMessages, abortController, (token) => {
			currentReply = currentReply + token
		})

		messages = [...messages, { role: 'assistant', content: currentReply }]

		isSubmitting = false
	}

	function submitSuggestion(suggestion: string) {
		inputValue = suggestion
		handleSubmit()
	}

	function renderMarkdownLinks(content: string) {
		const linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g
		return content.replace(
			linkRegex,
			'<a href="$2" target="_blank" rel="noopener noreferrer" class="text-blue-400 dark:text-blue-300 hover:underline">$1</a>'
		)
	}

	function resetChat() {
		console.log('resetChat')
		abortController.abort()
		abortController = new AbortController()

		messages = [
			{
				role: 'assistant',
				content: firstMessage
			}
		]
		inputValue = ''
		currentReply = ''
		isSubmitting = false
	}

	$effect(() => {
		if ($globalChatInitialInput.length > 0) {
			inputValue = $globalChatInitialInput
			globalChatInitialInput.set('')
			handleSubmit()
		}
	})

	$effect(() => {
		messages = [
			{
				role: 'assistant',
				content: firstMessage
			}
		]
	})
</script>

<div class="relative flex flex-col h-full bg-surface z-20">
	<!-- Reset Button -->
	<div class="fixed top-3 right-3 flex flex-row justify-end gap-2">
		<Button
			buttonType="button"
			on:click={resetChat}
			disabled={!hasCopilot}
			startIcon={{ icon: Plus }}
			size="xs2"
			aria-label="Reset chat"
		>
			New chat
		</Button>
	</div>

	<!-- Chat Messages -->
	<div class="flex-1 overflow-y-auto p-4 space-y-4 z-10 mt-12">
		{#each messages as msg}
			<div class={twMerge('flex flex-col', msg.role === 'user' ? 'items-end' : 'items-start')}>
				<div
					class={twMerge(
						'max-w-[98%] p-3 rounded-lg text-sm',
						msg.role === 'user'
							? 'bg-blue-500 text-white rounded-br-sm'
							: 'bg-gray-100 dark:bg-gray-700 text-primary rounded-bl-sm'
					)}
				>
					<p class="whitespace-pre-wrap break-words"
						>{@html renderMarkdownLinks(msg.content as string)}</p
					>
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

		<!-- Suggestion buttons when no user messages yet -->
		{#if !hasUserMessages && !isSubmitting}
			<div class="w-full pt-4">
				<div class="flex flex-wrap gap-2">
					{#each suggestions as suggestion}
						<Button
							on:click={() => submitSuggestion(suggestion)}
							size="xs2"
							color="blue"
							buttonType="button"
							disabled={!hasCopilot}
							btnClasses="whitespace-normal text-left"
						>
							{suggestion}
						</Button>
					{/each}
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
				disabled={!hasCopilot || isSubmitting}
			></textarea>
			<Button
				size="md"
				disabled={!hasCopilot || !inputValue.trim() || isSubmitting}
				iconOnly
				startIcon={{ icon: Send }}
				on:click={handleSubmit}
			/>
		</div>
	</div>
</div>
