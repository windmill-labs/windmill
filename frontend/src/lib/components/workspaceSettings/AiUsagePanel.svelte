<script lang="ts">
	import { AiService, type AITokenUsageBucket, type ModelPriceOverride } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { formatUsd, priceSpend, type ModelSpend } from '../copilot/modelPricing'
	import { formatTokenCount } from '../copilot/chat/tokenUsage'
	import SettingCard from '../instanceSettings/SettingCard.svelte'
	import Select from '../select/Select.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { resource } from 'runed'

	let { modelPricing }: { modelPricing: Record<string, ModelPriceOverride> } = $props()

	type GroupBy = 'day' | 'user' | 'model' | 'session'

	let days = $state(30)
	let groupBy = $state<GroupBy>('day')

	const rangeOptions = [
		{ label: 'Last 7 days', value: 7 },
		{ label: 'Last 30 days', value: 30 },
		{ label: 'Last 90 days', value: 90 }
	]

	let usage = resource(
		() => ({ workspace: $workspaceStore, days, groupBy }),
		async ({ workspace, days, groupBy }) =>
			workspace ? await AiService.listAiUsage({ workspace, days, groupBy }) : []
	)

	// The API groups by (dimension, provider, model) so every bucket resolves to a
	// single rate; the table folds those back into one line per dimension value.
	type Bucket = ModelSpend & { key: string; requests: number }

	function toSpend(bucket: AITokenUsageBucket): Bucket {
		return {
			// Grouping by model has no separate dimension — the model is the key.
			key: groupBy === 'model' ? `${bucket.provider}/${bucket.model}` : bucket.key || '—',
			requests: bucket.requests,
			provider: bucket.provider,
			model: bucket.model,
			tokens: {
				input: bucket.input_tokens,
				cacheRead: bucket.cache_read_tokens,
				cacheWrite: bucket.cache_write_tokens,
				output: bucket.output_tokens
			},
			reportedCostUsd:
				bucket.reported_cost_nano_usd != undefined
					? bucket.reported_cost_nano_usd / 1_000_000_000
					: undefined
		}
	}

	let priced = $derived(priceSpend((usage.current ?? []).map(toSpend), modelPricing))

	let rows = $derived.by(() => {
		const byKey = new Map<
			string,
			{ key: string; cost: number | undefined; tokensIn: number; tokensOut: number; requests: number }
		>()
		for (const row of priced.rows) {
			const existing = byKey.get(row.key) ?? {
				key: row.key,
				cost: undefined,
				tokensIn: 0,
				tokensOut: 0,
				requests: 0
			}
			existing.tokensIn += row.tokens.input + row.tokens.cacheRead + row.tokens.cacheWrite
			existing.tokensOut += row.tokens.output
			existing.requests += row.requests
			if (row.cost !== undefined) {
				existing.cost = (existing.cost ?? 0) + row.cost
			}
			byKey.set(row.key, existing)
		}
		return [...byKey.values()].sort((a, b) => (b.cost ?? 0) - (a.cost ?? 0))
	})
</script>

<SettingCard
	label="AI usage"
	description="Token spend across this workspace's AI chats. Costs are estimated from the rates above unless the provider reported one."
>
	<div class="flex flex-col gap-3">
		<div class="flex flex-row items-center gap-2 flex-wrap">
			<div class="w-40">
				<Select items={rangeOptions} bind:value={days} />
			</div>
			<ToggleButtonGroup bind:selected={groupBy}>
				{#snippet children({ item })}
					<ToggleButton value="day" label="By day" {item} />
					<ToggleButton value="user" label="By user" {item} />
					<ToggleButton value="model" label="By model" {item} />
					<ToggleButton value="session" label="By session" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>

		{#if usage.loading}
			<p class="text-xs text-tertiary">Loading…</p>
		{:else if usage.error}
			<p class="text-xs text-tertiary">
				Could not load usage. Only workspace admins can read it.
			</p>
		{:else if rows.length === 0}
			<p class="text-xs text-tertiary">No AI usage recorded in this period.</p>
		{:else}
			<div class="flex flex-row items-baseline gap-2">
				<span class="text-lg font-semibold tabular-nums">{formatUsd(priced.total)}</span>
				<span class="text-xs text-tertiary">
					total{priced.hasUnpriced ? ', excluding models with no price' : ''}
				</span>
			</div>
			<div class="overflow-x-auto border rounded-md">
				<table class="w-full text-xs">
					<thead class="bg-surface-secondary">
						<tr class="text-left text-secondary">
							<th class="px-3 py-2 font-medium">{groupBy}</th>
							<th class="px-3 py-2 font-medium text-right">In</th>
							<th class="px-3 py-2 font-medium text-right">Out</th>
							<th class="px-3 py-2 font-medium text-right">Requests</th>
							<th class="px-3 py-2 font-medium text-right">Cost</th>
						</tr>
					</thead>
					<tbody>
						{#each rows as row (row.key)}
							<tr class="border-t">
								<td class="px-3 py-2 font-mono truncate max-w-xs">{row.key}</td>
								<td class="px-3 py-2 text-right tabular-nums">{formatTokenCount(row.tokensIn)}</td>
								<td class="px-3 py-2 text-right tabular-nums">{formatTokenCount(row.tokensOut)}</td>
								<td class="px-3 py-2 text-right tabular-nums">{row.requests}</td>
								<td class="px-3 py-2 text-right tabular-nums">
									{row.cost === undefined ? 'no rate' : formatUsd(row.cost)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</SettingCard>
