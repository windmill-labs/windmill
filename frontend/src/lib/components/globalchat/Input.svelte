<script lang="ts">
	import { chatRequest, prepareUserMessage, prepareSystemMessage } from './core'

	// Using Svelte 5 runes for reactivity
	let inputValue = $state('')
	let isSubmitting = $state(false)
	let currentReply = $state('')

	let abortController = new AbortController()

	// Props definition using $props
	let { placeholder = 'Type a message...', buttonText = 'Send' } = $props()

	async function handleSubmit() {
		if (!inputValue.trim()) return

		isSubmitting = true
		currentReply = ''

		const userMessage = prepareUserMessage(inputValue)
		const systemMessage = prepareSystemMessage()
		let messages = [systemMessage]
		messages.push({ role: 'user', content: userMessage })

		const result = await chatRequest(messages, abortController, (token) => {
			currentReply = currentReply + token
		})

		console.log(result)
		console.log(currentReply)

		// Reset the input field
		inputValue = ''
		isSubmitting = false
	}
</script>

<div class="flex flex-col gap-2">
	<form
		class="flex w-full gap-2"
		onsubmit={(e) => {
			e.preventDefault()
			handleSubmit()
		}}
	>
		<input
			type="text"
			class="flex-1 px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
			bind:value={inputValue}
			{placeholder}
			disabled={isSubmitting}
		/>

		<button
			type="submit"
			class="px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
			disabled={isSubmitting || !inputValue.trim()}
		>
			{buttonText}
		</button>
	</form>
	<div class="flex flex-row border rounded-md p-2">
		<p>{currentReply}</p>
	</div>
</div>
