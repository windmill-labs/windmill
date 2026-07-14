<script lang="ts">
	import { copilotInfo } from '$lib/aiStore'
	import UsageMeter from './UsageMeter.svelte'

	// Shows how much of the one-time free Windmill AI grant is spent, using the same gauge
	// as the context-usage indicator. Only while actively on the free tier: an exhausted
	// grant is surfaced by the chat's dedicated banner/overlay instead.
	let freeTier = $derived($copilotInfo.freeTier)
	let visible = $derived(!!freeTier && !freeTier.exhausted)
	let fillPct = $derived(Math.round(Math.min(freeTier?.used_ratio ?? 0, 1) * 100))

	// Green while there is plenty left, amber past 80% (running low), red near the end —
	// mirroring the context gauge's escalate-toward-the-limit palette.
	let fillClass = $derived(
		fillPct >= 95 ? 'bg-red-500' : fillPct >= 80 ? 'bg-amber-500' : 'bg-surface-accent-primary'
	)
</script>

{#if visible}
	<UsageMeter {fillPct} {fillClass} ariaLabel="Free Windmill AI usage">
		{#snippet tooltip()}
			<div class="text-xs whitespace-nowrap">
				<p class="font-semibold">Free Windmill AI</p>
				<p class="mt-1 tabular-nums">{fillPct}% used</p>
				<p class="mt-1 text-tertiary">
					One-time allowance. Add your own API key for unlimited use.
				</p>
			</div>
		{/snippet}
	</UsageMeter>
{/if}
