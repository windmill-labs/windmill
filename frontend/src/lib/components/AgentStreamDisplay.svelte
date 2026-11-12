<script lang="ts">
	import { onDestroy, untrack } from 'svelte'
	import AgentTrace from './AgentTrace.svelte'
	import type { AgentTraceEntry } from './agentTrace'
	import { prefersInstantReveal, TypewriterReveal } from './copilot/chat/typewriterReveal'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import { runPane } from './agentScroll'
	import { createBottomSticker } from './stickToBottom'
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
			if (!continues) {
				currentLane.restart()
				reasoningLane.restart()
			}
			currentLane.feed(progress.stream.current)
			reasoningLane.feed(progress.stream.reasoning)
			if (!continues) {
				// Another run, so the reader's decision to stop following the previous
				// one does not carry over: this one starts at its latest output.
				following = true
			}
		})
	})

	// The buffer lands in bursts, once per poll of the job, so the live text is shown as a
	// prefix a typewriter advances, as the chat does for its own replies. Finished entries
	// are shown whole: a tool call settles that text, and pacing it under the card would
	// reorder the trace.
	function revealLane() {
		let revealed = $state(0)
		let fed = ''
		const reveal = new TypewriterReveal({
			instant: prefersInstantReveal(),
			// The chat's run card polls once a second; a shorter lag types out each burst
			// and then stalls until the next.
			smoothingMs: 1000,
			onReveal: (chunk) => (revealed += chunk.length)
		})
		function restart() {
			reveal.reset()
			revealed = 0
			fed = ''
		}
		return {
			get revealed() {
				return revealed
			},
			feed(text: string) {
				// The text only grows within a turn; anything else is the next turn's.
				if (!text.startsWith(fed)) {
					restart()
				}
				reveal.push(text.slice(fed.length))
				fed = text
			},
			restart,
			reset: () => reveal.reset()
		}
	}
	const currentLane = revealLane()
	const reasoningLane = revealLane()
	onDestroy(() => {
		currentLane.reset()
		reasoningLane.reset()
	})

	let stream = $derived({
		entries: progress.stream.entries,
		current: progress.stream.current.slice(0, currentLane.revealed),
		reasoning: progress.stream.reasoning.slice(0, reasoningLane.revealed)
	})

	const traceEntries: AgentTraceEntry[] = $derived(
		progress.stream.entries.map((entry) =>
			entry.kind === 'tool'
				? {
						kind: 'tool',
						name: entry.name,
						args: entry.args,
						result: entry.result,
						running: entry.running,
						failed: entry.success === false
					}
				: entry
		)
	)

	let anchor: HTMLElement | undefined = $state()
	// Carry the reader along as the text is written, unless they have scrolled up
	// to read something — then the pane is theirs until they come back to the end.
	let following = true
	const sticker = createBottomSticker()

	// The listener goes on the pane, not on this element: a scroll event fires on
	// whatever actually scrolled and does not bubble, so a handler here would never
	// run and the reader would be dragged back on every poll.
	$effect(() => {
		const pane = runPane(anchor)
		if (!pane) {
			return
		}
		// Told apart by direction, not by `isOwnScroll`'s time window: the typewriter scrolls
		// every frame, so every reader scroll would land inside that window and be undone.
		// The end is checked first: content shrinking (reasoning replaced by the answer or a
		// tool row) also moves the pane up, but leaves it clamped at the end.
		let lastTop = pane.scrollTop
		const onScroll = () => {
			if (sticker.isAtEnd(pane)) {
				following = true
			} else if (pane.scrollTop < lastTop) {
				following = false
			}
			lastTop = pane.scrollTop
		}
		pane.addEventListener('scroll', onScroll, { passive: true })
		return () => pane.removeEventListener('scroll', onScroll)
	})

	$effect(() => {
		stream.current
		stream.reasoning
		stream.entries.length
		if (following) {
			sticker.scrollToEnd(runPane(anchor))
		}
	})
</script>

<!-- One gap on the container rather than margins per child, and the same gap the
     finished trace uses: a turn's text must not shift when a tool call turns it from
     the live text into a row. The text at the bottom is deliberately unlabelled — a
     turn that goes on to call a tool was narration, and the run's end settles which. -->
<div bind:this={anchor} class="flex flex-col w-full py-3 gap-1">
	{#if traceEntries.length > 0}
		<!-- The finished result's renderer, so a row reads the same while the run streams.
		     The landed result mounts its own trace: rows opened here close then, and a
		     failed call's label is red only here, since the finished trace has no outcome
		     for a row until its job is loaded. -->
		<AgentTrace entries={traceEntries} runKey={streamKey ?? ''} />
	{/if}

	{#if stream.current !== ''}
		<div>
			<!-- Same sanitizing chain as a finished output: a partial answer is written
			     by the same model and is no more trusted for arriving in pieces. -->
			<GfmMarkdown md={stream.current} noPadding />
		</div>
	{:else if stream.reasoning !== ''}
		<!-- Reasoning arrives before the text, so on its own it means the model is
		     still thinking rather than that this run has nothing to say. -->
		<div class="text-secondary">
			<GfmMarkdown md={stream.reasoning} prose="xs" noPadding />
		</div>
	{/if}
</div>
