<script lang="ts">
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import SessionStatusDot from '$lib/components/sessions/SessionStatusDot.svelte'
	import type { SessionChatStatus } from '$lib/components/sessions/sessionRuntime.svelte'
	import UnreadCountBadge from '$lib/components/common/badge/UnreadCountBadge.svelte'
	import { PencilLine } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Item } from '$lib/utils'

	/**
	 * One conversation in a list of them: the AI session sidebar's row shape, with the resting
	 * mark a chat needs (a test run or one of the deployed flow's). Shared by the chat panel's
	 * own sidebar and by any page that lists conversations beside a chat.
	 */
	interface Props {
		title: string
		status?: SessionChatStatus
		isTest?: boolean
		selected?: boolean
		/** Why the row cannot be opened, when something stops it. Disables it and says why. */
		lockedReason?: string
		unread?: number
		/** A message is waiting to be sent in this chat. */
		queued?: boolean
		/** Keeps the row's menu visible while one of its actions is under way. */
		busy?: boolean
		onSelect: () => void
		/** The row's own actions, shown in a menu on hover. */
		actions?: () => Item[]
	}

	let {
		title,
		status = 'idle',
		isTest = false,
		selected = false,
		lockedReason = undefined,
		unread = 0,
		queued = false,
		busy = false,
		onSelect,
		actions = undefined
	}: Props = $props()
</script>

<Button
	unifiedSize="md"
	variant="subtle"
	onClick={onSelect}
	{selected}
	disabled={!!lockedReason}
	title={lockedReason}
	btnClasses="transition-all duration-150 group gap-2"
>
	<!-- Says what the chat is doing where there is something to say, and which kind of chat it
	     is otherwise. -->
	<SessionStatusDot
		{status}
		isFork={false}
		restingTitle={isTest ? 'Test chat, run from the flow editor' : 'Chat on the deployed flow'}
	>
		{#snippet resting()}
			<span
				class="w-[6px] h-[6px] rounded-full {isTest
					? 'border border-gray-400 dark:border-gray-500'
					: 'bg-gray-300 dark:bg-gray-600'}"
			></span>
		{/snippet}
	</SessionStatusDot>
	<span
		class={twMerge('flex-1 text-left truncate', unread > 0 ? 'font-semibold text-primary' : '')}
	>
		{title}
	</span>
	{#if queued || unread > 0}
		<span class="shrink-0 inline-flex items-center gap-1">
			{#if queued}
				<PencilLine class="w-3 h-3 text-tertiary" aria-label="Message waiting to send" />
			{/if}
			<UnreadCountBadge count={unread} />
		</span>
	{/if}
	<!-- Hidden while the row is disabled: it sits inside the row's button, and a disabled button
	     swallows every click in its subtree, so a visible menu here would be an affordance that
	     does nothing. -->
	{#if actions && !lockedReason}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class={twMerge(
				'ml-2 transition-all duration-100 opacity-0 group-hover:opacity-100',
				busy ? 'opacity-100' : ''
			)}
			onclick={(e) => e.stopPropagation()}
		>
			<DropdownV2 items={actions} size="xs" />
		</div>
	{/if}
</Button>
