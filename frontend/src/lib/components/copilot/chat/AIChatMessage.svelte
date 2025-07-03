<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { DisplayMessage } from './shared'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import AssistantMessage from './AssistantMessage.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCwIcon, Undo2Icon } from 'lucide-svelte'
	import AIChatInput from './AIChatInput.svelte'
	import type { ContextElement } from './context'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		message: DisplayMessage
		messageIndex: number
	}

	let { message, messageIndex, availableContext, selectedContext = $bindable() }: Props = $props()

	let editingMessageIndex = $state<number | null>(null)

	function startEditMessage(messageIndex: number) {
		editingMessageIndex = messageIndex
	}
</script>

<div
	class={twMerge(
		message.role === 'user' && messageIndex > 0 && 'mt-6',
		'mb-2',
		message.role !== 'user' ? 'cursor-default' : 'cursor-pointer'
	)}
	role="button"
	tabindex="0"
	onclick={() => (message.role === 'user' ? startEditMessage(messageIndex) : null)}
	onkeydown={() => {}}
>
	{#if message.role === 'user' && message.contextElements && editingMessageIndex !== messageIndex}
		<div class="flex flex-row gap-1 mb-1 overflow-scroll no-scrollbar px-2">
			{#each message.contextElements as element}
				<ContextElementBadge contextElement={element} />
			{/each}
		</div>
	{/if}
	{#if message.role === 'user' && editingMessageIndex === messageIndex}
		<AIChatInput
			{availableContext}
			bind:selectedContext
			initialInstructions={message.content}
			bind:editingMessageIndex
		/>
	{:else}
		<div
			class={twMerge(
				'text-sm py-1 mx-2',
				message.role === 'user' &&
					'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg relative group',
				(message.role === 'assistant' || message.role === 'tool') && 'px-[1px]',
				message.role === 'tool' && 'text-tertiary',
				editingMessageIndex !== null && 'opacity-50'
			)}
		>
			{#if message.role === 'assistant'}
				<AssistantMessage {message} />
			{:else}
				{message.content}
			{/if}
		</div>
	{/if}
	{#if message.role === 'user' && message.snapshot}
		<div class="mx-2 text-sm text-tertiary flex flex-row items-center justify-between gap-2 mt-2">
			Saved a flow snapshot
			<Button
				size="xs2"
				variant="border"
				color="light"
				on:click={() => {
					if (message.snapshot) {
						aiChatManager.flowAiChatHelpers?.revertToSnapshot(message.snapshot)
					}
				}}
				title="Revert to snapshot"
				startIcon={{ icon: Undo2Icon }}
			>
				Revert
			</Button>
		</div>
	{/if}
</div>
{#if message.role === 'user' && message.error}
	<div class="flex justify-end px-2 -mt-1">
		<Button
			size="xs2"
			variant="border"
			title="Retry generation"
			color="light"
			startIcon={{ icon: RefreshCwIcon }}
			onclick={() => aiChatManager.retryRequest()}
		>
			Retry
		</Button>
	</div>
{/if}
