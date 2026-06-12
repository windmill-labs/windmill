<script lang="ts">
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { copilotInfo, copilotSessionModel } from '$lib/aiStore'
	import { getModelContextWindow } from '../lib'
	import { getAiChatManager } from './aiChatManagerContext'

	const aiChatManager = getAiChatManager()

	let providerModel = $derived(
		$copilotSessionModel ?? $copilotInfo.defaultModel ?? $copilotInfo.aiModels[0]
	)
	let contextWindow = $derived(
		providerModel ? getModelContextWindow(providerModel.model) : undefined
	)
	let usedTokens = $derived(Math.round(aiChatManager.estimatedContextTokens))
	let pct = $derived(
		contextWindow ? Math.min(100, Math.round((usedTokens / contextWindow) * 100)) : undefined
	)
	let colorClass = $derived(
		pct !== undefined && pct >= 90
			? 'text-red-500'
			: pct !== undefined && pct >= 70
				? 'text-amber-500'
				: 'text-tertiary'
	)

	function formatTokenCount(tokens: number): string {
		if (tokens >= 1_000_000) {
			return `${(tokens / 1_000_000).toFixed(1)}M`
		}
		if (tokens >= 1000) {
			return `${Math.round(tokens / 1000)}k`
		}
		return `${tokens}`
	}
</script>

{#if pct !== undefined && contextWindow !== undefined && aiChatManager.messages.length > 0}
	<Tooltip small placement="top">
		<span class={`text-2xs tabular-nums ${colorClass}`} aria-label="Context window usage">
			{pct}%
		</span>
		{#snippet text()}
			<div class="max-w-56 text-xs">
				<p>
					Context window usage: ~{formatTokenCount(usedTokens)} / {formatTokenCount(
						contextWindow
					)} tokens
				</p>
				<p class="mt-1 text-2xs">
					{aiChatManager.contextUsage
						? 'Based on usage reported by the model on the last exchange.'
						: 'Rough estimate — refined once the model reports usage.'}
				</p>
			</div>
		{/snippet}
	</Tooltip>
{/if}
