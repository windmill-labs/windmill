<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Badge } from '$lib/components/common'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import AgentTrace from './AgentTrace.svelte'
	import ChatCollapsibleCard from './copilot/chat/ChatCollapsibleCard.svelte'
	import LabeledDivider from './LabeledDivider.svelte'
	import WebSearchSourcesDisplay from './copilot/chat/WebSearchSourcesDisplay.svelte'
	import { buildAgentTrace, splitFinalAnswer } from './agentTrace'
	import { runPane } from './agentScroll'
	import { createBottomSticker } from './stickToBottom'
	import { formatTokenCount, summarizeAgentResult, type AgentResult } from './aiAgentResult'

	interface Props {
		result: AgentResult
		workspaceId?: string
		/** Identifies the run, so a reused viewer lands on the new one's output. */
		runKey?: string
		/**
		 * How to render an output that is not text. An `output_schema` makes `output`
		 * an object, and the right rendering for it is whatever the result viewer
		 * would do with that object on its own — a table for rows, the file viewer
		 * for an S3 object. Passed in rather than imported so this component does not
		 * have to reach back into the viewer that renders it.
		 */
		structuredOutput: Snippet<[unknown]>
	}

	let { result, workspaceId, runKey, structuredOutput }: Props = $props()

	let summary = $derived(summarizeAgentResult(result))
	let textOutput = $derived(typeof result.output === 'string' ? result.output : undefined)

	// The turn that produced `output` is not a trace row: the output block below is
	// that same text, and printing it twice in one scroll reads as the agent having
	// answered itself. Its citations move down with it.
	let answer = $derived(splitFinalAnswer(buildAgentTrace(result.messages), result.output))
	let trace = $derived(answer.trace)
	let reasoning = $derived(result.reasoning?.trim())
	let reasoningExpanded = $state(false)

	let anchor: HTMLElement | undefined = $state()
	const sticker = createBottomSticker()
	$effect(() => {
		// Also on arrival from a stream: the run finishing adds the output separator,
		// so the end has moved from wherever the stream had the reader parked.
		runKey
		trace.length
		reasoning
		sticker.scrollToEnd(runPane(anchor))
	})
</script>

<div bind:this={anchor} class="flex flex-col w-full py-3">
	{#if trace.length > 0}
		<AgentTrace entries={trace} {workspaceId} />
	{/if}
	{#if reasoning}
		<!-- The whole run's thinking, which the worker hands back joined rather than
		     per iteration, so it reads as one block above the answer it led to. -->
		<ChatCollapsibleCard
			label="Thinking"
			expanded={reasoningExpanded}
			onToggle={() => (reasoningExpanded = !reasoningExpanded)}
			contentClass="font-main"
		>
			<GfmMarkdown md={reasoning} prose="xs" noPadding />
		</ChatCollapsibleCard>
	{/if}
	{#if trace.length > 0 || reasoning}
		<LabeledDivider class="my-3">
			<span class="text-2xs text-hint">Output</span>
		</LabeledDivider>
	{/if}
	<div>
		{#if textOutput !== undefined}
			{#if textOutput === ''}
				<span class="text-tertiary text-xs">The agent returned no answer</span>
			{:else}
				<!-- A model writes this, and what it writes is steerable by whatever
					     reached its context — a user message, a tool's output. So it is
					     untrusted input and goes through the shared sanitizing chain, which
					     is also what makes it inert on the public replay page. -->
				<GfmMarkdown md={textOutput} noPadding />
			{/if}
		{:else}
			{@render structuredOutput(result.output)}
		{/if}
		{#if answer.sources}
			<div class="mt-2">
				<WebSearchSourcesDisplay sources={answer.sources} />
			</div>
		{/if}
	</div>

	<!-- What the run cost, as a footnote to what it produced. -->
	<div class="flex items-center gap-2 flex-wrap text-xs mt-3">
		{#if summary.toolCalls > 0}
			<Badge color="blue">
				{summary.toolCalls}
				{summary.toolCalls === 1 ? 'tool call' : 'tool calls'}
			</Badge>
		{/if}
		{#if summary.webSearches > 0}
			<Badge color="blue">
				{summary.webSearches}
				{summary.webSearches === 1 ? 'web search' : 'web searches'}
			</Badge>
		{/if}
		{#if summary.tokens !== undefined}
			<Badge color="gray">{formatTokenCount(summary.tokens)} tokens</Badge>
		{/if}
		{#if summary.cachedTokens}
			<Badge color="gray">{formatTokenCount(summary.cachedTokens)} cached</Badge>
		{/if}
	</div>
</div>
