<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { DisplayMessage, ToolDisplayMessage } from './shared'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import AssistantMessage from './AssistantMessage.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCwIcon, Undo2Icon } from 'lucide-svelte'
	import AIChatInput from './AIChatInput.svelte'
	import type { ContextElement } from './context'
	import ToolExecutionDisplay from './ToolExecutionDisplay.svelte'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		message: DisplayMessage
		messageIndex: number
		editingMessageIndex: number | null
	}

	let {
		message,
		messageIndex,
		availableContext,
		selectedContext = $bindable(),
		editingMessageIndex = $bindable(null)
	}: Props = $props()

	function editMessage() {
		if (message.role !== 'user' || editingMessageIndex !== null || aiChatManager.loading) {
			return
		}
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
	onclick={() => editMessage()}
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
		<div class="px-2">
			<AIChatInput
				{availableContext}
				bind:selectedContext
				initialInstructions={message.content}
				{editingMessageIndex}
				onClickOutside={() => (editingMessageIndex = null)}
				onKeyDown={(e) => {
					if (e.key === 'Escape') {
						editingMessageIndex = null
					}
				}}
				onEditEnd={() => (editingMessageIndex = null)}
			/>
		</div>
	{:else}
		<div
			class={twMerge(
				'text-sm py-1 mx-2',
				message.role === 'user' &&
					'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg relative group',
				(message.role === 'assistant' || message.role === 'tool') && 'px-[1px]',
				message.role === 'tool' && 'text-tertiary'
			)}
		>
			{#if message.role === 'assistant'}
				<AssistantMessage {message} />
			{:else if message.role === 'tool'}
				<ToolExecutionDisplay message={message as ToolDisplayMessage} />
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
			onclick={() => aiChatManager.retryRequest(messageIndex)}
		>
			Retry
		</Button>
	</div>
{/if}
