<script lang="ts">
	import { untrack } from 'svelte'
	import ChatCollapsibleCard from './copilot/chat/ChatCollapsibleCard.svelte'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import { scrollPaneToEnd } from './agentScroll'
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

	let anchor: HTMLElement | undefined = $state()
	$effect(() => {
		// Follow the text as it is written, the same way the finished run opens on
		// its output: both put what you came for at the bottom.
		stream.answer
		stream.reasoning
		stream.tools.length
		scrollPaneToEnd(anchor)
	})
</script>

<!-- Deliberately the same order the finished run uses — what it did, then what it
     produced — so the view does not rearrange itself when the result lands. -->
<div class="flex flex-col w-full pt-1">
	{#each stream.tools as tool (tool.callId)}
		<ChatCollapsibleCard
			label={tool.name}
			expanded={false}
			toggleable={false}
			shimmer={tool.running}
			onToggle={() => {}}
			labelClass={tool.success === false ? 'text-red-500' : ''}
		/>
	{/each}

	<div class={stream.tools.length > 0 ? 'mt-4 pt-3 border-t border-border-light' : ''}>
		<span class="text-2xs text-hint">Output</span>
		<div class="mt-1">
			<!-- Same sanitizing chain as the finished output: a partial answer is
			     written by the same model and is no more trusted for arriving in
			     pieces. -->
			{#if stream.answer !== ''}
				<GfmMarkdown md={stream.answer} noPadding />
			{:else if stream.reasoning !== ''}
				<!-- Reasoning arrives before the answer, so on its own it means the model
				     is still thinking rather than that this run has no answer. -->
				<div class="text-secondary">
					<GfmMarkdown md={stream.reasoning} prose="xs" noPadding />
				</div>
			{/if}
		</div>
	</div>
	<div bind:this={anchor}></div>
</div>
