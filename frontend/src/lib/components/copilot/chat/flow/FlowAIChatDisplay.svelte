<script lang="ts">
	import autosize from '$lib/autosize'
	import { Button } from '$lib/components/common'
	import { Loader2, PlusIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import ProviderModelSelector from '../ProviderModelSelector.svelte'
	import type { AIChatContext, DisplayMessage } from '../shared'
	import { getContext } from 'svelte'
	import AssistantMessage from '../AssistantMessage.svelte'

	let {
		messages,
		instructions = $bindable(),
		sendRequest,
		clear
	}: {
		messages: DisplayMessage[]
		instructions: string
		sendRequest: () => void
		clear: () => void
	} = $props()

	let scrollEl: HTMLDivElement | undefined = $state()
	let automaticScroll = $state(true)

	export function enableAutomaticScroll() {
		automaticScroll = true
	}
	async function scrollDown() {
		scrollEl?.scrollTo({
			top: scrollEl.scrollHeight,
			behavior: 'smooth'
		})
	}

	let height = $state(0)
	$effect(() => {
		automaticScroll && height && scrollDown()
	})

	const { loading, currentReply } = getContext<AIChatContext>('AIChatContext')

	export function focusInput() {}
</script>

<div class="flex flex-col h-full">
	<div
		class="flex flex-row items-center justify-between gap-2 p-2 border-b border-gray-200 dark:border-gray-600"
	>
		<div class="flex flex-row items-center gap-2">
			<p class="text-sm font-semibold">Chat</p>
		</div>
		<div class="flex flex-row items-center gap-2">
			<Button
				title="New chat"
				on:click={() => {
					clear()
				}}
				size="md"
				btnClasses="!p-1"
				startIcon={{ icon: PlusIcon }}
				iconOnly
				variant="border"
				color="light"
			/>
		</div>
	</div>
	{#if messages.length > 0}
		<div
			class="h-full overflow-y-scroll pt-2"
			bind:this={scrollEl}
			onwheel={(e) => {
				automaticScroll = false
			}}
		>
			<div class="flex flex-col" bind:clientHeight={height}>
				{#each messages as message}
					<div
						class={twMerge(
							'text-sm py-1 mx-2',
							message.role === 'user' &&
								'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg mb-2',
							(message.role === 'assistant' || message.role === 'tool') && 'px-[1px]',
							message.role === 'tool' && 'text-gray-500'
						)}
					>
						{#if message.role === 'assistant'}
							<AssistantMessage {message} />
						{:else}
							{message.content}
						{/if}
					</div>
				{/each}
				{#if $loading && !$currentReply}
					<div class="mb-6 py-1 px-2">
						<Loader2 class="animate-spin" />
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<div class:border-t={messages.length > 0}>
		<div class="relative w-full px-2 scroll-pb-2 pt-2">
			<textarea
				bind:value={instructions}
				use:autosize
				onkeydown={(e) => {
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault()
						sendRequest()
					}
				}}
				rows={3}
				placeholder={messages.length === 0 ? 'Ask anything' : 'Ask followup'}
				class="resize-none"
			></textarea>
		</div>
		<div class={`flex flex-row justify-end items-center gap-2 px-0.5`}>
			<ProviderModelSelector />
		</div>
	</div>
</div>
