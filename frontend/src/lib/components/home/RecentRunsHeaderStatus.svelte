<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import { ChevronDown, ExternalLink, PanelRightOpen, Play } from 'lucide-svelte'
	import RecentRunRow from './RecentRunRow.svelte'
	import {
		allRunsHref,
		askAI,
		canAskAI,
		collisionPrefixes,
		totalLabel,
		windowLabel,
		type RecentRunsState
	} from './recentRuns.svelte'

	interface Props {
		runs: RecentRunsState
		folder: string | undefined
		/** Absent below the width the side panel needs: there is no panel to bring back. */
		onShowPanel?: () => void
		onFocusItem?: (path: string) => void
		/** A narrow bar: the trigger is one icon, its dot the colour of the worst status. */
		compact?: boolean
	}

	let { runs, folder, onShowPanel, onFocusItem, compact = false }: Props = $props()

	const MAX_ITEMS = 8

	let isOpen = $state(false)
	let data = $derived(runs.current)
	let failing = $derived(data?.items.filter((i) => i.status === 'failure') ?? [])
	let prefixes = $derived(data && !folder ? collisionPrefixes(data.items) : new Map())
</script>

<!-- Without data (still loading, or the request failed) the trigger stays: it is the only way
     back to a closed panel. -->
{#if data || onShowPanel}
	<div class="flex items-center h-[22px] pl-2.5 ml-1.5 border-l border-border-light">
		<Popover
			openOnHover
			debounceDelay={140}
			placement="bottom-end"
			bind:isOpen
			triggerAttrs={{ 'aria-label': 'Recent runs' }}
			class="flex items-center gap-2.5 h-7 pl-2 pr-1.5 -mr-1.5 rounded-md text-xs font-normal text-secondary whitespace-nowrap hover:bg-surface-secondary {isOpen
				? 'bg-surface-secondary'
				: ''}"
		>
			{#snippet trigger()}
				{#if compact}
					<span class="relative flex">
						<Play size={14} />
						{#if data}
							<span
								class="absolute -top-0.5 -right-1 w-[7px] h-[7px] rounded-full ring-2 ring-surface {data.failing_items >
								0
									? 'bg-red-500'
									: data.running_items > 0
										? 'bg-yellow-500'
										: 'bg-green-500'}"
							></span>
						{/if}
					</span>
				{:else}
					<span>{data ? totalLabel(data) : 'Recent runs'}</span>
					{#if data && data.failing_items > 0}
						<span class="flex items-center gap-1.5 text-red-600 dark:text-red-400">
							<span class="w-[7px] h-[7px] rounded-full bg-red-500"></span>
							{data.failing_items} failing
						</span>
					{:else if data}
						<span class="flex items-center gap-1.5">
							<span class="w-[7px] h-[7px] rounded-full bg-green-500"></span>
							all good
						</span>
					{/if}
					{#if data && data.running_items > 0}
						<span class="flex items-center gap-1.5 text-yellow-600 dark:text-yellow-400">
							<span class="w-[7px] h-[7px] rounded-full bg-yellow-500"></span>
							{data.running_items} running
						</span>
					{/if}
					<ChevronDown size={12} class="text-hint" />
				{/if}
			{/snippet}
			{#snippet content()}
				<div class="w-[380px] p-1.5 text-xs">
					{#if data}
						<div class="flex items-center gap-2 px-2.5 pt-2 pb-1.5 text-secondary">
							<b class="font-semibold text-emphasis text-[12.5px]">
								{windowLabel(data.window_hours)}
							</b>
							<span>
								{totalLabel(data)} ·
								<span class={data.failed_runs > 0 ? 'text-red-600 dark:text-red-400' : ''}>
									{data.failed_runs} failed
								</span>
							</span>
							{#if canAskAI() && failing.length > 0}
								<button
									class="ml-auto h-[26px] px-2.5 rounded text-xs font-medium text-ai bg-ai/10 hover:bg-ai/15"
									onclick={() =>
										askAI(
											{ kind: 'failing', items: failing },
											'Investigate why these items are failing'
										)}
								>
									Ask AI about these {failing.length}
								</button>
							{/if}
						</div>

						<!-- Every item the counts above are made of, in the panel's order: a list of only the
					     failing ones under a total of all runs reads as missing rows. -->
						{#each data.items.slice(0, MAX_ITEMS) as item (item.path)}
							<RecentRunRow
								{item}
								now={runs.now}
								prefix={prefixes.get(item.path)}
								strip={false}
								onSelect={onFocusItem &&
									((path) => {
										isOpen = false
										onFocusItem(path)
									})}
							/>
						{:else}
							<p class="px-2.5 py-3 text-secondary text-[12.5px]">
								Runs show up here once something runs.
							</p>
						{/each}
						{#if data.items.length > MAX_ITEMS}
							<a
								href={allRunsHref(folder)}
								class="block px-2.5 py-1 ml-7 text-[11.5px] text-accent"
							>
								{data.items.length - MAX_ITEMS} more
							</a>
						{/if}
					{/if}

					<div
						class="flex items-center gap-2 px-1.5 pb-1 {data
							? 'mt-1.5 pt-2 border-t border-border-light'
							: 'pt-1'}"
					>
						<a
							href={allRunsHref(folder)}
							class="flex items-center gap-1.5 h-[26px] px-2 rounded-md text-secondary hover:bg-surface-secondary hover:text-primary"
						>
							All runs
							<ExternalLink size={12} />
						</a>
						{#if onShowPanel}
							<div class="ml-auto">
								<Button
									variant="subtle"
									unifiedSize="sm"
									startIcon={{ icon: PanelRightOpen }}
									onclick={() => {
										isOpen = false
										onShowPanel()
									}}
								>
									Show runs on home
								</Button>
							</div>
						{/if}
					</div>
				</div>
			{/snippet}
		</Popover>
	</div>
{/if}
