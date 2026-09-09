<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import type { AgentStream } from './aiAgentResult'

	interface Props {
		stream: AgentStream
	}

	let { stream }: Props = $props()
</script>

<div class="flex flex-col gap-2 w-full">
	{#if stream.tool}
		<div class="flex items-center gap-2 text-secondary text-xs">
			{#if stream.tool.running}
				<Loader2 class="animate-spin shrink-0" size={14} />
			{/if}
			<span class="font-mono truncate">{stream.tool.name}</span>
		</div>
	{/if}
	<!-- Same sanitizing chain as the finished answer: a partial answer is written by
	     the same model and is no more trusted for arriving in pieces. -->
	{#if stream.answer !== ''}
		<GfmMarkdown md={stream.answer} noPadding />
	{:else if stream.reasoning !== ''}
		<!-- Reasoning arrives before the answer, so on its own it means the model is
		     still thinking rather than that this run has no answer. -->
		<div class="text-secondary">
			<GfmMarkdown md={stream.reasoning} prose="xs" noPadding />
		</div>
	{/if}
</div>
