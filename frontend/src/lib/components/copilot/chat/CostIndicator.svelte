<script lang="ts">
	import { copilotInfo } from '$lib/aiStore'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { formatUsd, priceSpend, type ModelSpend } from '../modelPricing'
	import { getAiChatManager } from './aiChatManagerContext'
	import { billedTokens, formatTokenCount } from './tokenUsage'

	const aiChatManager = getAiChatManager()

	let spend = $derived(
		Object.values(aiChatManager.usageByModel).map(
			(entry): ModelSpend => ({
				provider: entry.provider,
				model: entry.model,
				tokens: billedTokens(entry.usage),
				reportedCostUsd: entry.usage.cost
			})
		)
	)
	let priced = $derived(priceSpend(spend, $copilotInfo.modelPricing))
	let visible = $derived(priced.rows.length > 0)

	// A priced total of zero next to unpriced models would read as "this was free",
	// so a chat whose models have no rate shows no figure at all — the tooltip says
	// why and where to fix it.
	let label = $derived(
		priced.total === 0 && priced.hasUnpriced
			? '—'
			: `${formatUsd(priced.total)}${priced.hasUnpriced ? '+' : ''}`
	)
	let unpricedModels = $derived(priced.rows.filter((r) => r.cost === undefined))
	// Where each number came from, so an estimate is never mistaken for a bill. A
	// chat can mix the two (one model priced from a table, another billed back by
	// its provider), so the reported ones are marked per row and the caveat below
	// covers only the estimated remainder.
	let estimatedRows = $derived(
		priced.rows.filter((r) => r.source === 'builtin' || r.source === 'override')
	)
	let estimateSource = $derived(
		estimatedRows.some((r) => r.source === 'override')
			? 'Estimated from the rates set for this workspace.'
			: 'Estimated from list prices.'
	)
</script>

{#if visible}
	<Tooltip small placement="top">
		<div class="flex items-center h-5 px-1 text-2xs tabular-nums text-tertiary">
			{label}
		</div>
		{#snippet text()}
			<div class="text-xs whitespace-nowrap">
				<!-- One conversation's spend, not the session's: a session can hold several
				     conversations and each keeps its own usage. The workspace AI usage view
				     is where they add up per session. -->
				<p class="font-semibold">Chat cost</p>
				<div class="mt-1 flex flex-col gap-1">
					{#each priced.rows as row (`${row.provider}:${row.model}`)}
						<div>
							<p class="font-mono">{row.model}</p>
							<p class="tabular-nums text-tertiary">
								{formatTokenCount(row.tokens.input)} in
								{#if row.tokens.cacheRead > 0 || row.tokens.cacheWrite > 0}
									· {formatTokenCount(row.tokens.cacheRead + row.tokens.cacheWrite)} cached
								{/if}
								· {formatTokenCount(row.tokens.output)} out · {row.cost === undefined
									? 'no rate'
									: formatUsd(row.cost)}{row.source === 'reported' ? ' billed' : ''}
							</p>
						</div>
					{/each}
				</div>
				{#if priced.hasReported}
					<p class="mt-1 text-tertiary">"billed" is the amount the provider charged.</p>
				{/if}
				{#if estimatedRows.length > 0}
					<p class="mt-1 text-tertiary">{estimateSource}</p>
				{/if}
				{#if unpricedModels.length > 0}
					<p class="mt-1 text-tertiary">
						No price for {unpricedModels.map((r) => r.model).join(', ')}. Set one in workspace
						settings, under AI.
					</p>
				{/if}
			</div>
		{/snippet}
	</Tooltip>
{/if}
