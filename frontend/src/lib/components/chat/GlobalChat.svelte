<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Send, Loader2, Plus } from 'lucide-svelte'
	import { chatRequest, prepareSystemMessage } from './core'
	import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
	import { globalChatInitialInput, userStore, copilotInfo } from '$lib/stores'
	import AiChat from '../copilot/chat/AIChat.svelte'

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
	<AiChat navigatorMode />
</div>
