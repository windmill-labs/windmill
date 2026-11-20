<!--
@component
A recorded conversation with an agent, drawn with the AI chat's own read-only components.
Shared by the recorder and the Hub page so the two cannot show different things.
-->
<script lang="ts">
	import { untrack } from 'svelte'
	import AIChatMessage from '$lib/components/copilot/chat/AIChatMessage.svelte'
	import ToolGroupDisplay from '$lib/components/copilot/chat/ToolGroupDisplay.svelte'
	import ChatTypingIndicator from '$lib/components/copilot/chat/ChatTypingIndicator.svelte'
	import { setChatViewHost } from '$lib/components/copilot/chat/chatViewHost'
	import { readOnlyChatViewHost } from '$lib/components/copilot/chat/readOnlyChatViewHost.svelte'
	import { chatItemKey, groupToolRuns } from '$lib/components/copilot/chat/toolGroups'
	import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
	import { agentRecordingMessages } from './agentRecording'
	import type { AgentTurn } from './types'

	interface Props {
		turns: AgentTurn[]
		/** A message sent and not answered yet, shown after the turns with the typing dots. */
		pending?: string
		/** How many messages are on screen, for a replay; all of them when undefined. */
		count?: number
		/** A replay is running: the typing dots show while the next message is on its way. */
		playing?: boolean
		workspace?: string
	}

	let {
		turns,
		pending = undefined,
		count = undefined,
		playing = false,
		workspace = ''
	}: Props = $props()

	const all = $derived.by((): DisplayMessage[] => {
		const messages = agentRecordingMessages(turns)
		return pending === undefined
			? messages
			: [...messages, { role: 'user', index: messages.length, content: pending }]
	})

	// The host is handed this array once and reads it from then on, so it is kept and
	// rewritten in place. Untracked, or the effect would rerun on its own write.
	const shown: DisplayMessage[] = $state([])
	setChatViewHost(
		readOnlyChatViewHost({ messages: shown, workspace, artifacts: [], openArtifact: () => {} })
	)
	$effect(() => {
		const next = all.slice(0, count ?? all.length)
		untrack(() => shown.splice(0, shown.length, ...next))
	})

	const chatItems = $derived(groupToolRuns(shown))
	const waiting = $derived(
		pending !== undefined ||
			(playing && count !== undefined && count < all.length && all[count]?.role !== 'user')
	)
</script>

{#snippet messageRow(message: DisplayMessage, messageIndex: number, inGroup: boolean)}
	<AIChatMessage
		{message}
		{messageIndex}
		availableContext={[]}
		editingMessageIndex={null}
		isLast={messageIndex === shown.length - 1}
		{inGroup}
		showAnswerActions={false}
	/>
{/snippet}
{#snippet groupRow(message: DisplayMessage, messageIndex: number)}
	{@render messageRow(message, messageIndex, true)}
{/snippet}

<div class="flex flex-col pb-2">
	{#each chatItems as item (chatItemKey(item))}
		{#if item.kind === 'group'}
			<div class="mb-1 min-w-0 px-2 text-sm text-primary">
				<div class="px-[1px]">
					<ToolGroupDisplay group={item} entry={groupRow} />
				</div>
			</div>
		{:else}
			{@render messageRow(item.message, item.index, false)}
		{/if}
	{/each}
	{#if waiting}
		<div class="px-2">
			<ChatTypingIndicator loading compact />
		</div>
	{/if}
</div>
