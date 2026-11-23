<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ChevronRight, Eye } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import { itemKey, itemTitle, type RecentRunsState } from './recentRuns.svelte'

	interface Props {
		runs: RecentRunsState
		class?: string
	}

	let { runs, class: classNames = '' }: Props = $props()

	let hidden = $derived(runs.hiddenItems)
	let open = $state(false)
</script>

{#if hidden.length > 0}
	<div class={twMerge('flex flex-col text-[11px] text-hint', classNames)}>
		<div class="flex items-center gap-1 h-6">
			<Button
				variant="subtle"
				unifiedSize="2xs"
				btnClasses="!text-[11px] !text-hint !px-1 whitespace-nowrap"
				wrapperClasses="-ml-1"
				startIcon={{
					icon: ChevronRight,
					classes: twMerge('transition-transform', open ? 'rotate-90' : '')
				}}
				aria-expanded={open}
				onClick={() => (open = !open)}
			>
				{hidden.length} hidden item{hidden.length === 1 ? '' : 's'}
			</Button>
			{#if open && hidden.length > 1}
				<Button
					variant="subtle"
					unifiedSize="2xs"
					btnClasses="!text-[11px] whitespace-nowrap"
					wrapperClasses="ml-auto"
					onClick={() => runs.unhide(hidden)}
				>
					Unhide all
				</Button>
			{/if}
		</div>
		{#if open}
			<ul transition:slide|local={{ duration: 100 }} class="flex flex-col">
				{#each hidden as item (itemKey(item))}
					<li
						class="flex items-center gap-2 min-h-[30px] pl-5 pr-0.5 rounded-md hover:bg-surface-hover"
					>
						<span class="flex flex-col min-w-0 flex-1">
							<span class="truncate text-[12px] text-secondary">{itemTitle(item)}</span>
							<span class="truncate text-[10.5px]" title={item.path}>{item.path}</span>
						</span>
						<Button
							variant="subtle"
							unifiedSize="2xs"
							startIcon={{ icon: Eye }}
							btnClasses="!text-[11px] shrink-0"
							title="Show {itemTitle(item)} in Recently run again"
							onClick={() => runs.unhide([item])}
						>
							Unhide
						</Button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}
