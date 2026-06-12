<script lang="ts">
	import { copilotInfo, copilotSessionModel } from '$lib/aiStore'
	import { getKnownModelContextWindow } from '../modelConfig'
	import { getAiChatManager } from './aiChatManagerContext'

	const aiChatManager = getAiChatManager()

	let providerModel = $derived(
		$copilotSessionModel ?? $copilotInfo.defaultModel ?? $copilotInfo.aiModels[0]
	)
	let contextWindow = $derived(
		providerModel ? getKnownModelContextWindow(providerModel.model) : undefined
	)
	// Last provider-reported usage — a fact, though one turn stale (it doesn't
	// grow until the next completion reports). Nothing is shown before the
	// first report.
	let usedTokens = $derived(Math.round(aiChatManager.contextUsage ?? 0))
	// With a known window, only surface once the conversation actually fills it;
	// without one there is no threshold to compare against, so always show.
	let visible = $derived(
		usedTokens > 0 &&
			aiChatManager.messages.length > 0 &&
			(contextWindow === undefined || usedTokens >= contextWindow * 0.5)
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
	<div class="flex justify-end px-1">
		<span class="text-[0.6rem] text-tertiary tabular-nums" aria-label="Context window usage">
			context window usage: ~{formatTokenCount(usedTokens)}{contextWindow
				? ` / ${formatTokenCount(contextWindow)}`
				: ''}
		</span>
	</div>
{/if}
