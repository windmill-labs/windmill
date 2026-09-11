<script lang="ts">
	import { untrack } from 'svelte'
	import ChatCollapsibleCard from './copilot/chat/ChatCollapsibleCard.svelte'
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

<!-- The same order a finished run uses — what it did, then what it is saying — so
     nothing moves when the result lands. The text at the bottom is deliberately
     unlabelled: a turn that goes on to call a tool was narration, and only the end
     of the run settles which this one is. -->
<div class="flex flex-col w-full pt-3">
	{#each stream.entries as entry, index (entry.kind === 'tool' ? entry.callId : index)}
		{#if entry.kind === 'tool'}
			<ChatCollapsibleCard
				label={entry.name}
				expanded={false}
				toggleable={false}
				shimmer={entry.running}
				onToggle={() => {}}
				labelClass={entry.success === false ? 'text-red-500' : ''}
			/>
		{:else}
			<div class="mb-1">
				<GfmMarkdown md={entry.content} noPadding />
			</div>
		{/if}
	{/each}

	{#if stream.current !== ''}
		<div class="mt-2">
			<!-- Same sanitizing chain as a finished output: a partial answer is written
			     by the same model and is no more trusted for arriving in pieces. -->
			<GfmMarkdown md={stream.current} noPadding />
		</div>
	{:else if stream.reasoning !== ''}
		<!-- Reasoning arrives before the text, so on its own it means the model is
		     still thinking rather than that this run has nothing to say. -->
		<div class="text-secondary mt-2">
			<GfmMarkdown md={stream.reasoning} prose="xs" noPadding />
		</div>
	{/if}
</div>
