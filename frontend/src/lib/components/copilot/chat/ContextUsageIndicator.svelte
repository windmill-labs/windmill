<script lang="ts">
	import { copilotInfo, copilotSessionModel } from '$lib/aiStore'
	import { getKnownModelContextWindow } from '../modelConfig'
	import { getAiChatManager } from './aiChatManagerContext'
	import { AIMode } from './AIChatManager.svelte'
	import UsageMeter from './UsageMeter.svelte'

	const aiChatManager = getAiChatManager()

	// The `/compact` slash command is only wired up in session-chat GLOBAL mode,
	// so only advertise it where it actually works.
	let canCompact = $derived(aiChatManager.isSessionChat && aiChatManager.mode === AIMode.GLOBAL)

	let providerModel = $derived(
		$copilotSessionModel ?? $copilotInfo.defaultModel ?? $copilotInfo.aiModels[0]
	)
	let contextWindow = $derived(
		providerModel ? getKnownModelContextWindow(providerModel.model) : undefined
	)
	// The same number the compaction trigger uses: the provider's report when
	// one describes the current history (one turn stale by nature), otherwise
	// a live chars/4 estimate of the stored context.
	let usedTokens = $derived(Math.round(aiChatManager.contextTokens))
	// Always surface usage once a conversation has started, at any fill level, so
	// the user can watch context grow toward the compaction threshold. Hidden on the
	// free tier, where the free-usage meter takes this slot instead.
	let visible = $derived(
		usedTokens > 0 && aiChatManager.messages.length > 0 && !$copilotInfo.freeTier
	)

	// Compaction triggers at 80% of the window (COMPACTION_TRIGGER_RATIO); the
	// gauge fills toward that point and turns red once it is reached.
	const COMPACTION_TRIGGER_RATIO = 0.8
	let ratio = $derived(contextWindow ? Math.min(usedTokens / contextWindow, 1) : undefined)
	let fillPct = $derived(ratio !== undefined ? Math.round(ratio * 100) : undefined)
	let fillClass = $derived(
		ratio === undefined
			? 'bg-tertiary'
			: ratio >= COMPACTION_TRIGGER_RATIO
				? 'bg-red-500'
				: ratio >= COMPACTION_TRIGGER_RATIO * 0.75
					? 'bg-amber-500'
					: 'bg-surface-accent-primary'
	)

	function formatTokenCount(tokens: number): string {
		if (tokens >= 1_000_000) {
			return `${(tokens / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
		}
		if (tokens >= 1000) {
			return `${Math.round(tokens / 1000)}k`
		}
		return `${tokens}`
	}
</script>

{#if visible}
	<!-- Only a meter when we know the window: it's a 0–100% reading. With an unknown
	     window there's no max to measure against, so it's a plain labeled indicator
	     (the bar is decorative/full and the token count lives in the tooltip). -->
	<UsageMeter {fillPct} {fillClass} ariaLabel="Context window usage">
		{#snippet tooltip()}
			<div class="text-xs whitespace-nowrap">
				<p class="font-semibold">Context usage</p>
				<p class="mt-1 tabular-nums">
					~{formatTokenCount(usedTokens)}{contextWindow
						? ` / ${formatTokenCount(contextWindow)}`
						: ''}{fillPct !== undefined ? ` (${fillPct}%)` : ''}
				</p>
				{#if ratio !== undefined && ratio >= COMPACTION_TRIGGER_RATIO}
					<p class="mt-1 text-tertiary">History will be compacted soon to free up space.</p>
				{/if}
				{#if canCompact}
					<p class="mt-1 text-tertiary">
						Type <span class="font-mono">/compact</span> to summarize and free up space now.
					</p>
				{/if}
			</div>
		{/snippet}
	</UsageMeter>
{/if}
