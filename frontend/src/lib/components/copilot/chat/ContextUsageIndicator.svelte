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
	// The same number the compaction trigger uses: the provider's report when
	// one describes the current history (one turn stale by nature), otherwise
	// a live chars/4 estimate of the stored context.
	let usedTokens = $derived(Math.round(aiChatManager.contextTokens))
	// Always surface usage once a conversation has started, at any fill level, so
	// the user can watch context grow toward the compaction threshold.
	let visible = $derived(usedTokens > 0 && aiChatManager.messages.length > 0)

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
		<span class="text-[0.6rem] text-tertiary tabular-nums" aria-label="Context window usage">
			context window usage: ~{formatTokenCount(usedTokens)}{contextWindow
				? ` / ${formatTokenCount(contextWindow)}`
				: ''}
		</span>
{/if}
