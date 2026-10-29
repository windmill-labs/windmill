<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ExternalLink, PanelRightClose } from 'lucide-svelte'
	import RecentRunRow from './RecentRunRow.svelte'
	import {
		allRunsHref,
		askAI,
		canAskAI,
		collisionPrefixes,
		itemKey,
		totalLabel,
		windowLabel,
		type RecentRunItem,
		type RecentRunsState
	} from './recentRuns.svelte'

	interface Props {
		runs: RecentRunsState
		folder: string | undefined
		onCollapse: () => void
		onFocusItem?: (item: RecentRunItem) => void
	}

	let { runs, folder, onCollapse, onFocusItem }: Props = $props()

	let data = $derived(runs.current)
	let windowText = $derived(data ? windowLabel(data.window_hours) : '')
	let prefixes = $derived(data && !folder ? collisionPrefixes(data.items) : new Map())
</script>

<div class="flex flex-col h-full min-h-0">
	<div class="flex items-center gap-2 shrink-0 pb-2">
		<h2 class="flex items-baseline gap-1.5 min-w-0 flex-1 text-[13px] font-medium text-primary">
			<span class="whitespace-nowrap">Recently run</span>
			{#if folder}
				<span class="shrink-0 max-w-[120px] truncate font-medium text-secondary text-xs"
					>in {folder}</span
				>
			{/if}
			{#if data}
				<span
					class="relative min-w-0 truncate text-xs font-normal text-hint"
					title="{windowText} · {totalLabel(data)} · {data.failed_runs} failed"
				>
					{#if data.failed_runs > 0}
						<span class="group/failed relative">
							<span class="text-red-600 dark:text-red-400">{data.failed_runs}</span> failed
							{#if canAskAI()}
								<button
									class="absolute left-full top-1/2 -translate-y-1/2 ml-1 h-5 px-1.5 rounded text-[11px] font-medium text-ai bg-ai/10 hover:bg-ai/15 whitespace-nowrap opacity-0 pointer-events-none group-hover/failed:opacity-100 group-hover/failed:pointer-events-auto focus:opacity-100 focus:pointer-events-auto"
									onclick={() =>
										askAI(
											{
												kind: 'failing',
												items: data.items.filter((i) => i.status === 'failure')
											},
											'Investigate why these items are failing'
										)}
								>
									Ask AI
								</button>
							{/if}
						</span>
					{:else}
						{totalLabel(data)}
					{/if}
					<!-- Room goes to the folder name first; the window stays in the tooltip. -->
					<span class="max-[1180px]:hidden {folder ? 'hidden' : ''}"> · {windowText}</span>
				</span>
			{/if}
		</h2>
		<a
			href={allRunsHref(folder)}
			class="flex items-center gap-1 h-[26px] px-2 rounded-md text-xs text-secondary whitespace-nowrap hover:bg-surface-hover hover:text-primary"
		>
			All runs
			<ExternalLink size={12} />
		</a>
		<Button
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: PanelRightClose }}
			onclick={onCollapse}
			aria-label="Hide recent runs"
			title="Hide recent runs"
		/>
	</div>

	<div class="flex-1 min-h-0 overflow-y-auto overscroll-contain -mx-2.5">
		{#if data && data.items.length === 0}
			<p class="px-2.5 text-xs text-hint">Runs show up here once something runs.</p>
		{:else if data}
			{#each data.items as item (itemKey(item))}
				<RecentRunRow
					{item}
					now={runs.now}
					prefix={prefixes.get(itemKey(item))}
					onSelect={onFocusItem}
				/>
			{/each}
		{/if}
	</div>
</div>
