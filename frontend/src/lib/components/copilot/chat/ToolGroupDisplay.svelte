<script lang="ts">
	import type { Snippet } from 'svelte'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import type { DisplayMessage, ToolDisplayMessage } from './shared'
	import { callFailed, groupHeader, type ToolGroup } from './toolGroups'
	import McpServerIcon from '$lib/components/mcp/McpServerIcon.svelte'
	import { resolveMcpServerMark } from '$lib/components/mcp/serverMark'
	import { Button } from '$lib/components/common'
	import { getChatViewHost } from './chatViewHost'

	interface Props {
		group: ToolGroup
		entry: Snippet<[DisplayMessage, number]>
	}

	let { group, entry }: Props = $props()

	const chatHost = getChatViewHost()

	let expanded = $state(false)
	const rows: Record<number, HTMLElement> = {}
	let scrollOnceOpen = false

	function scrollToFailedCall() {
		const failed = group.entries.find((e) => e.message.role === 'tool' && callFailed(e.message))
		if (failed) {
			// Here rather than at the click: while the group opens above a chat pinned to its end,
			// scroll anchoring keeps it at the end, and that scroll turns automatic scroll back on.
			chatHost.disableAutomaticScroll()
			rows[failed.index]?.scrollIntoView({ block: 'center' })
		}
	}

	function revealFailedCall() {
		if (expanded) {
			scrollToFailedCall()
		} else {
			scrollOnceOpen = true
			expanded = true
		}
	}

	const calls = $derived(
		group.entries.map((e) => e.message).filter((m): m is ToolDisplayMessage => m.role === 'tool')
	)
	// Same states as a single row: a group whose calls are all still waiting their turn is faded
	// like a queued row. Once one call has run, the group is in progress until every call has,
	// including the moments between two calls when none is executing.
	const executing = $derived(calls.some((m) => m.isLoading || m.isStreamingArguments))
	const waiting = $derived(calls.some((m) => m.isQueued))
	const queued = $derived(!executing && waiting && calls.every((m) => m.isQueued))
	const running = $derived(executing || (waiting && !queued))
	const header = $derived(groupHeader(group, running ? 'running' : queued ? 'queued' : 'settled'))
	const failedCount = $derived(calls.filter(callFailed).length)
	// Every edit of one item carries the same chip, so the group shows it once.
	const previewCard = $derived(calls.findLast((m) => m.previewCard && !m.error)?.previewCard)
	// An explore header has a prefix only when every call is an MCP read on one server; that
	// group is marked with the server's icon, as a single MCP row is.
	const mcpServer = $derived(
		group.groupKind === 'explore' && header.prefix
			? calls.find((m) => m.mcpServer)?.mcpServer
			: undefined
	)
	// What the group is doing right now, so a collapsed run still names its current step.
	const liveLine = $derived(
		running
			? (
					calls.findLast((m) => m.isLoading || m.isStreamingArguments) ??
					calls.find((m) => m.isQueued)
				)?.content
			: undefined
	)
</script>

{#snippet serverMark()}
	{#if mcpServer}
		{#await resolveMcpServerMark(mcpServer.workspace, mcpServer.path) then mark}
			<McpServerIcon icon={mark.icon} size={14} />
		{/await}
	{/if}
{/snippet}

{#snippet status()}
	<div class="flex items-center gap-2 shrink-0">
		{#if failedCount > 0}
			<Button
				variant="subtle"
				unifiedSize="2xs"
				destructive
				btnClasses="font-main text-2xs text-red-600 dark:text-red-400 hover:bg-red-100 hover:text-red-800 dark:hover:bg-red-700/40 dark:hover:text-red-100"
				title="Show the failed call"
				onClick={revealFailedCall}
			>
				{failedCount} failed
			</Button>
		{/if}
		{#if previewCard}
			<ToolPreviewCard card={previewCard} />
		{/if}
	</div>
{/snippet}

<ChatCollapsibleCard
	labelPrefix={header.prefix}
	label={header.label}
	labelMore={header.more}
	{expanded}
	onToggle={() => {
		scrollOnceOpen = false
		expanded = !expanded
	}}
	onContentShown={() => {
		if (scrollOnceOpen) {
			scrollOnceOpen = false
			scrollToFailedCall()
		}
	}}
	shimmer={running}
	settleLabel
	{liveLine}
	class={queued ? 'opacity-60 hover:opacity-100 transition-opacity' : ''}
	labelClass="truncate"
	headerRight={failedCount > 0 || previewCard ? status : undefined}
	headerLeft={mcpServer ? serverMark : undefined}
	contentClass="border-0 border-l rounded-none bg-transparent p-0 pl-2 ml-1.5 mt-0.5"
>
	{#each group.entries as { message, index } (index)}
		<div bind:this={rows[index]}>
			{@render entry(message, index)}
		</div>
	{/each}
</ChatCollapsibleCard>
