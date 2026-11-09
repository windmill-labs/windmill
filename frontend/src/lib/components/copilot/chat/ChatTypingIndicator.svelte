<script lang="ts">
	import { Hourglass } from 'lucide-svelte'
	import { formatElapsed } from './liveElapsed'

	let {
		loading,
		compact = false,
		paused = false,
		label
	}: { loading: boolean; compact?: boolean; paused?: boolean; label?: string } = $props()

	// Starts on the rising edge of `loading`, frozen on its last value once loading
	// ends so a caller reading it just after still sees a coherent number. `paused`
	// suspends it and resumes where it stopped: time the user spends answering is
	// theirs, and counting it makes a fast turn read as a slow one.
	let elapsedMs = $state(0)
	let accumulatedMs = 0
	let wasLoading = false
	$effect(() => {
		if (!loading) {
			wasLoading = false
			return
		}
		if (!wasLoading) {
			wasLoading = true
			accumulatedMs = 0
			elapsedMs = 0
		}
		if (paused) return
		const startedAt = Date.now() - accumulatedMs
		const interval = setInterval(() => (elapsedMs = Date.now() - startedAt), 1000)
		return () => {
			accumulatedMs = Date.now() - startedAt
			clearInterval(interval)
		}
	})
</script>

{#if paused}
	<span
		class={(compact ? 'gap-1 px-1.5 py-0.5 text-[10px]' : 'gap-1.5 px-2 py-1 text-2xs') +
			' inline-flex items-center rounded-md bg-surface/80 backdrop-blur text-accent'}
		aria-label="Waiting for your input"
	>
		<Hourglass class={(compact ? 'w-2.5 h-2.5' : 'w-3 h-3') + ' hourglass-flip'} />
		Waiting for your input
	</span>
{:else}
	<span
		class={compact
			? 'inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-surface/80 backdrop-blur'
			: 'inline-flex items-center gap-2 px-2 py-1 rounded-md bg-surface/80 backdrop-blur'}
		aria-label="AI is generating a response"
	>
		<span class={compact ? 'inline-flex items-end gap-0.5' : 'inline-flex items-end gap-1'}>
			<span
				class={(compact ? 'w-[3px] h-[3px]' : 'w-[5px] h-[5px]') +
					' rounded-full bg-accent chat-typing-dot'}
			></span>
			<span
				class={(compact ? 'w-[3px] h-[3px]' : 'w-[5px] h-[5px]') +
					' rounded-full bg-accent chat-typing-dot chat-typing-dot-2'}
			></span>
			<span
				class={(compact ? 'w-[3px] h-[3px]' : 'w-[5px] h-[5px]') +
					' rounded-full bg-accent chat-typing-dot chat-typing-dot-3'}
			></span>
		</span>
		<span
			class={(compact ? 'text-[10px]' : 'text-2xs') + ' text-tertiary tabular-nums leading-none'}
			>{label ? label + ' · ' : ''}{formatElapsed(elapsedMs)}</span
		>
	</span>
{/if}

<style>
	.chat-typing-dot {
		animation: chat-typing 1.2s ease-in-out infinite;
	}
	.chat-typing-dot-2 {
		animation-delay: 0.15s;
	}
	.chat-typing-dot-3 {
		animation-delay: 0.3s;
	}
	@keyframes chat-typing {
		0%,
		60%,
		100% {
			opacity: 0.3;
		}
		30% {
			opacity: 1;
		}
	}

	/* Hourglass flips every 4s with long rests at each upright position. Global:
	   the class lands on a lucide component's own element. */
	:global(.hourglass-flip) {
		animation: hourglass-flip 4s cubic-bezier(0.65, 0, 0.35, 1) infinite;
		transform-origin: center;
	}
	@keyframes hourglass-flip {
		0%,
		35% {
			transform: rotate(0deg);
		}
		50%,
		85% {
			transform: rotate(180deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
