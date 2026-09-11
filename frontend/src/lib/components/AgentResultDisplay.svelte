<script lang="ts">
	import { Bot } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import { Badge } from '$lib/components/common'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import AgentTranscript from './AgentTranscript.svelte'
	import { formatTokenCount, summarizeAgentResult, type AgentResult } from './aiAgentResult'

	interface Props {
		result: AgentResult
		/** Answer or the conversation behind it; JSON is the viewer's own toggle. */
		view: 'answer' | 'transcript'
		workspaceId?: string
		/**
		 * How to render an answer that is not text. An `output_schema` makes `output`
		 * an object, and the right rendering for it is whatever the result viewer
		 * would do with that object on its own — a table for rows, the file viewer
		 * for an S3 object. Passed in rather than imported so this component does not
		 * have to reach back into the viewer that renders it.
		 */
		structuredOutput: Snippet<[unknown]>
	}

	let { result, view, workspaceId, structuredOutput }: Props = $props()

	let summary = $derived(summarizeAgentResult(result))
	let textOutput = $derived(typeof result.output === 'string' ? result.output : undefined)
</script>

<!-- pt-2 clears the toggle group and control bar the result viewer puts directly
     above this, which otherwise sit on the meta line. -->
<div class="flex flex-col gap-2 w-full pt-2">
	<div class="flex items-center gap-2 flex-wrap text-xs">
		<Bot size={14} class="text-tertiary shrink-0" />
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

	{#if view === 'transcript'}
		<AgentTranscript messages={result.messages} {workspaceId} />
	{:else if textOutput !== undefined}
		{#if textOutput === ''}
			<span class="text-tertiary text-xs">The agent returned no answer</span>
		{:else}
			<!-- A model writes this answer, and what it writes is steerable by whatever
			     reached its context — a user message, a tool's output. So it is
			     untrusted input and goes through the shared sanitizing chain, which is
			     also what makes it inert on the public replay page. -->
			<GfmMarkdown md={textOutput} noPadding />
		{/if}
	{:else}
		{@render structuredOutput(result.output)}
	{/if}
</div>
