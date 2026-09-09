<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Loader2 } from 'lucide-svelte'
	import { markdownProse } from './markdownProse'
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
	{#if stream.answer !== ''}
		<div class={markdownProse.sm}>
			<Markdown md={stream.answer} plugins={[gfmPlugin()]} />
		</div>
	{:else if stream.reasoning !== ''}
		<!-- Reasoning arrives before the answer, so on its own it means the model is
		     still thinking rather than that this run has no answer. -->
		<div class="{markdownProse.xs} text-secondary">
			<Markdown md={stream.reasoning} plugins={[gfmPlugin()]} />
		</div>
	{/if}
</div>
