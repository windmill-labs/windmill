<script lang="ts">
	import { CircleX, Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import {
		advanceAgentStream,
		emptyAgentStreamProgress,
		type AgentStreamProgress
	} from './aiAgentResult'

	interface Props {
		/** The raw `result_stream` buffer, which only ever grows for a given run. */
		raw: string
		/** Identifies the run, so a viewer reused for another one starts over. */
		streamKey?: string
	}

	let { raw, streamKey }: Props = $props()

	let progress: AgentStreamProgress & { key?: string } = $state(emptyAgentStreamProgress())

	$effect(() => {
		raw
		streamKey
		untrack(() => {
			// A shorter buffer than what was already folded in cannot be a longer
			// version of the same stream, so treat it as a different one.
			const continues = progress.key === streamKey && raw.length >= progress.consumed
			const base = continues ? progress : emptyAgentStreamProgress()
			progress = { ...advanceAgentStream(raw, base), key: streamKey }
		})
	})

	let stream = $derived(progress.stream)
</script>

<div class="flex flex-col gap-2 w-full">
	{#if stream.tool}
		<div
			class="flex items-center gap-2 text-xs {stream.tool.success === false
				? 'text-red-500'
				: 'text-secondary'}"
		>
			{#if stream.tool.running}
				<Loader2 class="animate-spin shrink-0" size={14} />
			{:else if stream.tool.success === false}
				<CircleX class="shrink-0" size={14} />
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
