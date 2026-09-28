<script lang="ts">
	import type { Snippet } from 'svelte'
	import { dismissTiming, sessionsBetaBanner } from './sessionsBetaBannerState.svelte'

	// Wraps the assistant settings button: it pulses once when the sessions beta
	// banner is dismissed, pointing at where the way back to the legacy chat went.
	let { children }: { children: Snippet } = $props()
</script>

<span class="relative inline-flex">
	{#if sessionsBetaBanner.hintSettings}
		<span
			class="pointer-events-none absolute inset-0 rounded-md bg-surface-accent-primary opacity-75 motion-safe:animate-ping-once"
			style="animation-duration: {dismissTiming.pulseMs}ms"
		></span>
	{/if}
	{@render children()}
</span>
