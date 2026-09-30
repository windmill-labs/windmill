<script lang="ts">
	import type { Snippet } from 'svelte'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import type { DisplayMessage, ToolDisplayMessage } from './shared'
	import { callFailed, groupHeader, type ToolGroup } from './toolGroups'
	import McpServerIcon from '$lib/components/mcp/McpServerIcon.svelte'
	import { resolveMcpServerMark } from '$lib/components/mcp/serverMark'

	interface Props {
		group: ToolGroup
		entry: Snippet<[DisplayMessage, number]>
	}

	let { group, entry }: Props = $props()

	let expanded = $state(false)

	const calls = $derived(
		group.entries.map((e) => e.message).filter((m): m is ToolDisplayMessage => m.role === 'tool')
	)
	// Same states as a single row: only an executing call shimmers and makes the header read as
	// in progress, and a group whose calls are all still waiting their turn is faded like a
	// queued row.
	const running = $derived(calls.some((m) => m.isLoading || m.isStreamingArguments))
	const queued = $derived(!running && calls.some((m) => m.isQueued))
	const header = $derived(groupHeader(group, running))
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
		running ? calls.findLast((m) => m.isLoading || m.isStreamingArguments)?.content : undefined
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
			<span class="font-main text-2xs text-red-600 dark:text-red-400">{failedCount} failed</span>
		{/if}
		{#if previewCard}
			<ToolPreviewCard card={previewCard} />
		{/if}
	</div>
{/snippet}

<ChatCollapsibleCard
	labelPrefix={header.prefix}
	label={header.label}
	{expanded}
	onToggle={() => (expanded = !expanded)}
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
		{@render entry(message, index)}
	{/each}
</ChatCollapsibleCard>
