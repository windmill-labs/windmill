<script lang="ts">
	let {
		loading,
		compact = false,
		label
	}: { loading: boolean; compact?: boolean; label?: string } = $props()

	// Wall-clock for the typing-dots indicator. Starts on the rising edge of
	// `loading`, ticks once a second, frozen on the last value when loading
	// ends so callers reading the dots briefly after still see a coherent number.
	let loadingStartedAt = $state<number | undefined>(undefined)
	let loadingElapsedMs = $state(0)
	$effect(() => {
		if (!loading) {
			loadingStartedAt = undefined
			return
		}
		loadingStartedAt = Date.now()
		loadingElapsedMs = 0
		const interval = setInterval(() => {
			if (loadingStartedAt) loadingElapsedMs = Date.now() - loadingStartedAt
		}, 1000)
		return () => clearInterval(interval)
	})

	function formatElapsed(ms: number): string {
		const total = Math.max(0, Math.floor(ms / 1000))
		if (total < 60) return `${total}s`
		const m = Math.floor(total / 60)
		const s = total % 60
		if (m < 60) return s === 0 ? `${m}m` : `${m}m ${s}s`
		const h = Math.floor(m / 60)
		const rm = m % 60
		return rm === 0 ? `${h}h` : `${h}h ${rm}m`
	}
</script>

<span
	class={compact
		? 'inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-surface/80 backdrop-blur'
		: 'inline-flex items-center gap-2 px-2 py-1 rounded-md bg-surface/80 backdrop-blur'}
	aria-label="AI is generating a response"
>
	<span class={compact ? 'inline-flex items-end gap-0.5' : 'inline-flex items-end gap-1'}>
		<span class={(compact ? 'w-1 h-1' : 'w-1.5 h-1.5') + ' rounded-full bg-accent chat-typing-dot'}
		></span>
		<span
			class={(compact ? 'w-1 h-1' : 'w-1.5 h-1.5') +
				' rounded-full bg-accent chat-typing-dot chat-typing-dot-2'}
		></span>
		<span
			class={(compact ? 'w-1 h-1' : 'w-1.5 h-1.5') +
				' rounded-full bg-accent chat-typing-dot chat-typing-dot-3'}
		></span>
	</span>
	<span class={(compact ? 'text-[10px]' : 'text-2xs') + ' text-tertiary tabular-nums leading-none'}
		>{label ? label + ' · ' : ''}{formatElapsed(loadingElapsedMs)}</span
	>
</span>

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
</style>
