<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import { ChevronDown, ExternalLink, PanelRightOpen } from 'lucide-svelte'
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
	}

	let { runs, folder, onShowPanel }: Props = $props()

	const MAX_FAILING = 5
	const MAX_RUNNING = 3

	let isOpen = $state(false)
	let data = $derived(runs.current)
	let failing = $derived(data?.items.filter((i) => i.status === 'failure') ?? [])
	let running = $derived(data?.items.filter((i) => i.status === 'running') ?? [])
	let prefixes = $derived(data && !folder ? collisionPrefixes(data.items) : new Map())
	let failingHref = $derived(
		`/runs?status=failure${folder ? `&folder=${encodeURIComponent(folder)}` : ''}`
	)
</script>

{#if data}
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
				<span>{totalLabel(data)}</span>
				{#if data.failing_items > 0}
					<span class="flex items-center gap-1.5 text-red-600 dark:text-red-400">
						<span class="w-[7px] h-[7px] rounded-full bg-red-500"></span>
						{data.failing_items} failing
					</span>
				{:else}
					<span class="flex items-center gap-1.5">
						<span class="w-[7px] h-[7px] rounded-full bg-green-500"></span>
						all good
					</span>
				{/if}
				{#if data.running_items > 0}
					<span class="flex items-center gap-1.5 text-yellow-600 dark:text-yellow-400">
						<span class="w-[7px] h-[7px] rounded-full bg-yellow-500"></span>
						{data.running_items} running
					</span>
				{/if}
				<ChevronDown size={12} class="text-hint" />
			{/snippet}
			{#snippet content()}
				<div class="w-[380px] p-1.5 text-xs">
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
						{#if canAskAI && failing.length > 0}
							<button
								class="ml-auto h-[26px] px-2.5 rounded text-xs font-medium text-ai bg-ai/10 hover:bg-ai/15"
								onclick={() =>
									askAI(
										{ kind: 'failing', paths: failing.map((i) => i.path) },
										'Investigate why these items are failing'
									)}
							>
								Ask AI about these {failing.length}
							</button>
						{/if}
					</div>

					{#if failing.length > 0}
						<div
							class="px-2.5 pt-2 pb-1 text-[11px] font-medium uppercase tracking-wide text-red-600 dark:text-red-400"
						>
							Failing <span class="normal-case tracking-normal font-normal"
								>{data.failing_items} {data.failing_items === 1 ? 'item' : 'items'}</span
							>
						</div>
						{#each failing.slice(0, MAX_FAILING) as item (item.path)}
							<RecentRunRow {item} now={runs.now} prefix={prefixes.get(item.path)} strip={false} />
						{/each}
						{#if data.failing_items > MAX_FAILING}
							<a href={failingHref} class="block px-2.5 py-1 ml-7 text-[11.5px] text-accent">
								{data.failing_items - MAX_FAILING} more failing
							</a>
						{/if}
					{/if}

					{#if running.length > 0}
						<div class="px-2.5 pt-2 pb-1 text-[11px] font-medium uppercase tracking-wide text-hint">
							Running <span class="normal-case tracking-normal font-normal"
								>{data.running_items} {data.running_items === 1 ? 'item' : 'items'}</span
							>
						</div>
						{#each running.slice(0, MAX_RUNNING) as item (item.path)}
							<RecentRunRow {item} now={runs.now} prefix={prefixes.get(item.path)} strip={false} />
						{/each}
					{/if}

					{#if failing.length === 0 && running.length === 0}
						<p class="px-2.5 py-3 text-secondary text-[12.5px]">
							{data.total_runs > 0 ? 'Nothing failing.' : 'Runs show up here once something runs.'}
						</p>
					{/if}

					<div class="flex items-center gap-2 mt-1.5 pt-2 px-1.5 pb-1 border-t border-border-light">
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
