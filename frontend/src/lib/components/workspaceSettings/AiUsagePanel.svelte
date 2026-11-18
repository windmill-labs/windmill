<script lang="ts">
	import { AiService, ApiError, type AITokenUsageBucket, type ModelPriceOverride } from '$lib/gen'
	import { formatUsd, priceSpend, type ModelSpend } from '../copilot/modelPricing'
	import { formatTokenCount } from '../copilot/chat/tokenUsage'
	import SettingCard from '../instanceSettings/SettingCard.svelte'
	import Select from '../select/Select.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { resource } from 'runed'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Cell from '../table/Cell.svelte'

	// Workspace and rates are both passed in rather than read from a store: the
	// settings component that mounts this one also serves the instance scope, and
	// the rates that priced a chat are the workspace's *effective* ones, which an
	// inheriting workspace does not hold itself.
	let {
		workspace,
		modelPricing,
		scope = 'workspace'
	}: {
		workspace: string
		modelPricing: Record<string, ModelPriceOverride>
		scope?: 'workspace' | 'self'
	} = $props()

	type GroupBy = 'day' | 'user' | 'model'

	let days = $state(30)
	let groupBy = $state<GroupBy>('day')

	const rangeOptions = [
		{ label: 'Last 7 days', value: 7 },
		{ label: 'Last 30 days', value: 30 },
		{ label: 'Last 90 days', value: 90 }
	]

	let usage = resource(
		() => ({ workspace, days, groupBy, scope }),
		async ({ workspace, days, groupBy, scope }) =>
			workspace ? await AiService.listAiUsage({ workspace, days, groupBy, scope }) : undefined
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

	let priced = $derived(priceSpend((usage.current?.buckets ?? []).map(toSpend), modelPricing))

	type Row = {
		key: string
		cost: number | undefined
		/** Every model behind this line was billed back by its provider, so the
		 * figure is an invoice rather than an estimate. A line mixing sources — or
		 * one holding a model with no rate, whose spend the figure omits entirely —
		 * makes the weaker claim. */
		reported: boolean
		tokensIn: number
		tokensOut: number
		requests: number
	}

	// Only a 403 on the workspace scope is a permission problem; reading your own
	// usage is open to any member. Attributing every failure to permissions sends an
	// admin looking for access they already hold, and buries the real cause of the
	// far more common transient ones (an expired session, a database hiccup).
	function usageError(error: unknown): string {
		if (scope === 'workspace' && error instanceof ApiError && error.status === 403) {
			return 'Only workspace admins can read workspace usage.'
		}
		return 'Could not load usage. Try again in a moment.'
	}

	// The headline sums both kinds, so it only escapes the ~ when nothing under it
	// was estimated.
	let totalIsEstimated = $derived(
		priced.rows.some((row) => row.cost !== undefined && row.source !== 'reported')
	)

	let rows = $derived.by(() => {
		const byKey = new Map<string, Row>()
		for (const row of priced.rows) {
			const existing = byKey.get(row.key) ?? {
				key: row.key,
				cost: undefined,
				reported: true,
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
			existing.reported &&= row.source === 'reported'
			byKey.set(row.key, existing)
		}
		return [...byKey.values()].sort((a, b) => (b.cost ?? 0) - (a.cost ?? 0))
	})
</script>

<SettingCard
	label={scope === 'self' ? `Your AI usage in ${workspace}` : 'AI usage'}
	description={scope === 'self'
		? "Token spend from your own AI chats in this workspace. Costs are estimated from the workspace's model rates unless the provider reported one."
		: "Token spend across this workspace's AI chats, grouped by day, user or model. Costs are estimated from the workspace's effective model rates unless the provider reported one."}
>
	<div class="flex flex-col gap-3">
		<div class="flex flex-row items-center justify-between gap-3 flex-wrap">
			<div class="flex flex-row items-center gap-2 flex-wrap">
				<div class="w-40">
					<Select items={rangeOptions} bind:value={days} />
				</div>
				<ToggleButtonGroup noWFull bind:selected={groupBy}>
					{#snippet children({ item })}
						<ToggleButton value="day" label="By day" {item} />
						{#if scope !== 'self'}
							<ToggleButton value="user" label="By user" {item} />
						{/if}
						<ToggleButton value="model" label="By model" {item} />
					{/snippet}
				</ToggleButtonGroup>
			</div>
			{#if !usage.loading && !usage.error && rows.length > 0}
				<div
					class="flex flex-row items-baseline gap-2"
					title={priced.hasUnpriced
						? 'Models with no rate set are not counted, so this figure is lower than the real spend.'
						: undefined}
				>
					<span class="text-lg font-semibold tabular-nums">
						{priced.total === 0 && priced.hasUnpriced
							? '—'
							: `${totalIsEstimated ? '~' : ''}${formatUsd(priced.total)}`}
					</span>
					<span class="text-xs text-tertiary">
						{usage.current?.truncated ? 'across the rows below' : 'total'}{priced.hasUnpriced
							? ' (partial)'
							: ''}
					</span>
				</div>
			{/if}
		</div>

		{#if usage.loading}
			<p class="text-xs text-tertiary">Loading…</p>
		{:else if usage.error}
			<p class="text-xs text-tertiary">{usageError(usage.error)}</p>
		{:else if rows.length === 0}
			<p class="text-xs text-tertiary">No AI usage recorded in this period.</p>
		{:else}
			{#if usage.current?.truncated}
				<p class="text-xs text-tertiary">
					More rows matched than are shown; the highest-volume ones are listed. Narrow the range
					or group differently to see the rest.
				</p>
			{/if}
			<DataTable size="sm" noBorder={false} rounded={true}>
				<Head>
					<tr>
						<Cell head first>{groupBy}</Cell>
						<Cell head numeric>In</Cell>
						<Cell head numeric>Out</Cell>
						<Cell head numeric>Requests</Cell>
						<Cell head numeric last>
							<span class="inline-flex flex-row items-center gap-1">
								Cost
								<Tooltip small placement="left">
									{#snippet text()}
										A cost marked ~ is estimated from this workspace's model rates. A cost
										without one was returned by the provider's API for those requests, and is used
										as is. "no rate" means the model has no price set, so its spend stays out of
										the total.
									{/snippet}
								</Tooltip>
							</span>
						</Cell>
					</tr>
				</Head>
				<tbody>
					{#each rows as row (row.key)}
						<tr class="border-b last:border-b-0">
							<Cell first class="font-mono truncate max-w-xs text-primary">{row.key}</Cell>
							<Cell numeric class="tabular-nums text-secondary"
								>{formatTokenCount(row.tokensIn)}</Cell
							>
							<Cell numeric class="tabular-nums text-secondary"
								>{formatTokenCount(row.tokensOut)}</Cell
							>
							<Cell numeric class="tabular-nums text-secondary">{row.requests}</Cell>
							<Cell numeric last class="tabular-nums text-primary">
								{row.cost === undefined
									? 'no rate'
									: `${row.reported ? '' : '~'}${formatUsd(row.cost)}`}
							</Cell>
						</tr>
					{/each}
				</tbody>
			</DataTable>
		{/if}
	</div>
</SettingCard>
