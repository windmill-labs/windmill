<script lang="ts">
	import { Button, Skeleton } from '$lib/components/common'
	import { ExternalLink, ListFilter, PanelRightClose, Sparkles } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import RecentRunRow from './RecentRunRow.svelte'
	import {
		allRunsHref,
		askAI,
		canAskAI,
		collisionPrefixes,
		failedRunsHref,
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
	// Rows' height (2.5 × 16px) with their gap: without it a first load reads as "nothing ran".
	const LOADING_LAYOUT = [[2.5], 0.25, [2.5], 0.25, [2.5]]
	// The failed count also holds runs an item has since recovered from; Ask AI attaches only
	// the items still failing, so it shows only when there is one.
	let failing = $derived(data?.items.filter((i) => i.status === 'failure') ?? [])
	// Every item the failed count is made of, recovered ones included.
	let failedItems = $derived(data?.items.filter((i) => i.failed_runs > 0) ?? [])
	let onlyFailing = $state(false)
	let failedMenuOpen = $state(false)
	// The menu holding the toggle goes once the last failure is resolved: the filter must go with it.
	let filtering = $derived(onlyFailing && failedItems.length > 0)
	$effect(() => {
		if (data && failedItems.length === 0) onlyFailing = false
	})
	let shownItems = $derived(filtering ? failedItems : (data?.items ?? []))
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
					class="flex items-baseline min-w-0 text-xs font-normal text-hint wm-page-in"
					title="{windowText} · {totalLabel(data)} · {data.failed_runs} failed"
				>
					{#if data.failed_runs > 0}
						<!-- A popover rather than a hover-revealed button: its content stays open while the
						     pointer travels to it, where a button beside the count vanished on the way. -->
						<Popover
							openOnHover
							debounceDelay={140}
							placement="bottom-start"
							bind:isOpen={failedMenuOpen}
							triggerAttrs={{ 'aria-label': `${data.failed_runs} failed runs` }}
							class="shrink-0 -mx-1 px-1 rounded whitespace-nowrap hover:bg-surface-hover {filtering ||
							failedMenuOpen
								? 'bg-surface-hover'
								: ''}"
						>
							{#snippet trigger()}
								<span class="text-red-600 dark:text-red-400">{data.failed_runs}</span> failed
							{/snippet}
							{#snippet content()}
								<div class="flex flex-col p-1 min-w-[200px] text-xs">
									<!-- Items past the listed ones can hold the whole count: nothing to filter to. -->
									{#if failedItems.length > 0}
										<Button
											variant="subtle"
											unifiedSize="sm"
											btnClasses="justify-start"
											startIcon={{ icon: ListFilter }}
											onclick={() => {
												onlyFailing = !onlyFailing
												failedMenuOpen = false
											}}
										>
											{filtering ? 'Show all items' : `Show only failed (${failedItems.length})`}
										</Button>
									{/if}
									{#if canAskAI() && failing.length > 0}
										<Button
											variant="subtle"
											unifiedSize="sm"
											btnClasses="justify-start text-ai hover:bg-ai/10"
											startIcon={{ icon: Sparkles }}
											onclick={() => {
												failedMenuOpen = false
												askAI(
													{ kind: 'failing', items: failing },
													'Investigate why these items are failing'
												)
											}}
										>
											Ask AI about {failing.length} failing item{failing.length === 1 ? '' : 's'}
										</Button>
									{/if}
									<Button
										variant="subtle"
										unifiedSize="sm"
										btnClasses="justify-start"
										startIcon={{ icon: ExternalLink }}
										href={failedRunsHref(folder, data.window_hours)}
									>
										View failed runs
									</Button>
								</div>
							{/snippet}
						</Popover>
					{:else}
						<span class="truncate">{totalLabel(data)}</span>
					{/if}
					<!-- Room goes to the folder name first; the window stays in the tooltip. -->
					<span class="truncate max-[1180px]:hidden {folder ? 'hidden' : ''}">
						&nbsp;· {windowText}</span
					>
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

	<div class="flex-1 min-h-0 overflow-y-auto overscroll-contain scrollbar-subtle -mx-2.5">
		{#if !data && runs.loading}
			<Skeleton class="px-2.5" layout={LOADING_LAYOUT} />
		{:else if !data && runs.failed}
			<p class="flex items-center gap-1 px-2.5 text-xs text-hint">
				Could not load recent runs.
				<Button variant="subtle" unifiedSize="xs" onClick={() => runs.retry()}>Retry</Button>
			</p>
		{:else if data && data.items.length === 0}
			<p class="px-2.5 text-xs text-hint">Runs show up here once something runs.</p>
		{:else if data}
			<!-- Fades in once, when the first read lands; refreshes update the rows in place. -->
			<div class="wm-page-in">
				{#each shownItems as item (itemKey(item))}
					<RecentRunRow
						{item}
						now={runs.now}
						prefix={prefixes.get(itemKey(item))}
						onResolve={runs.resolveFailures}
						onSelect={onFocusItem}
					/>
				{/each}
			</div>
		{/if}
	</div>
</div>
