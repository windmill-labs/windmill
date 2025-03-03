<script lang="ts">
	import autosize from '$lib/autosize'
	import { copilotInfo } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { AIChatHandler } from './core'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import CodeDisplay from './CodeDisplay.svelte'
	import { createEventDispatcher, setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import { Loader2 } from 'lucide-svelte'

	export let code: string
	export let lang: string | undefined = undefined

	const dispatch = createEventDispatcher<{
		applyCode: { code: string }
	}>()

	let loading = writable(false)
	let currentReply: Writable<string> = writable('')
	setContext<{
		originalCode: Writable<string>
		loading: Writable<boolean>
		currentReply: Writable<string>
		applyCode: (code: string) => void
	}>('AIChatContext', {
		originalCode: writable(code),
		loading,
		currentReply,
		applyCode: (code: string) => {
			dispatch('applyCode', { code })
		}
	})

	let instructions = ''

	$: chatHandler = new AIChatHandler($copilotInfo.ai_provider, lang ?? 'typescript')
	let messages: {
		role: 'user' | 'assistant'
		content: string
	}[] = []

	async function send() {
		currentReply.set('')
		$loading = true
		messages = [...messages, { role: 'user', content: instructions }]
		scrollDown()
		const oldInstructions = instructions
		instructions = ''
		await chatHandler.sendRequest(
			{
				instructions: oldInstructions,
				code
			},
			(token) => {
				currentReply.update((prev) => prev + token)
			}
		)
		messages = [...messages, { role: 'assistant', content: $currentReply }]
		currentReply.set('')
		$loading = false
		scrollDown()
	}

	let scrollEl: HTMLDivElement
	function scrollDown() {
		scrollEl?.scrollTo({
			top: scrollEl.scrollHeight,
			behavior: 'smooth'
		})
	}

	$: $currentReply && scrollDown()

	$: messagesWithCurrent = $currentReply
		? [...messages, { role: 'assistant', content: $currentReply }]
		: messages
</script>

<div class="flex flex-col h-full overflow-y-hidden">
	<div class="flex flex-col overflow-y-scroll pt-2 px-2" bind:this={scrollEl}>
		{#each messagesWithCurrent as message}
			<div
				class={twMerge(
					'text-sm py-1',
					message.role === 'user' &&
						'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg mb-2',
					message.role === 'assistant' && 'px-[1px] mb-6'
				)}
			>
				{#if message.role === 'assistant'}
					<div class="prose prose-sm dark:prose-invert">
						<Markdown
							md={message.content}
							plugins={[
								gfmPlugin(),
								{
									renderer: {
										pre: CodeDisplay
									}
								}
							]}
						/>
					</div>
				{:else}
					{message.content}
				{/if}
			</div>
		{/each}
		{#if $loading && !$currentReply}
			<div class="mb-6 py-1">
				<Loader2 class="animate-spin" />
			</div>
		{/if}
	</div>
	<div class="pb-2 px-2">
		<textarea
			on:keypress={(e) => {
				if (!instructions) {
					return
				}
				if (e.key === 'Enter' && !e.shiftKey) {
					e.preventDefault()
					send()
				}
			}}
			bind:value={instructions}
			use:autosize
			rows={3}
			placeholder={messages.length > 0 ? 'Ask followup' : 'Ask anything'}
			class="resize-none"
		/>
	</div>
</div>
