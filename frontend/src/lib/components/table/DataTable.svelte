<script context="module" lang="ts">
	export type DatatableContext = {
		size: 'xs' | 'sm' | 'md' | 'lg'
	}
</script>

<script lang="ts">
	import { createEventDispatcher, setContext } from 'svelte'
	import Button from '../common/button/Button.svelte'
	import { ArrowDownIcon, ArrowLeftIcon, ArrowRightIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import List from '$lib/components/common/layout/List.svelte'

	export let paginated: boolean = false
	export let currentPage: number = 1
	export let showNext: boolean = true
	export let loadMore: number = 0
	export let shouldLoadMore: boolean = false
	export let rounded: boolean = true
	export let size: 'xs' | 'sm' | 'md' | 'lg' = 'md'
	export let perPage: number | undefined = undefined
	export let shouldHidePagination: boolean = false
	export let noBorder: boolean = false
	export let rowCount: number | undefined = undefined
	export let hasMore: boolean = true
	export let contentHeight: number = 0

	let footerHeight: number = 0
	let tableHeight: number = 0
	const dispatch = createEventDispatcher()

	setContext<DatatableContext>('datatable', {
		size
	})

	$: contentHeight = tableHeight - footerHeight
</script>

<div
	class={twMerge('h-full', rounded ? 'rounded-md' : '', noBorder ? 'border-0' : 'border')}
	bind:clientHeight={tableHeight}
>
	<List justify="between" gap="none">
		<div class="w-full overflow-auto min-h-0 grow">
			<table class={twMerge('min-w-full divide-y')}>
				<slot />
			</table>
		</div>
		{#if paginated && !shouldHidePagination}
			<div
				class="w-full bg-surface border-t flex flex-row justify-between p-1 items-center gap-2 sticky bottom-0"
				bind:clientHeight={footerHeight}
			>
				<div>
					{#if rowCount}
						<span class="text-xs mx-2"> {rowCount} items</span>
					{/if}
				</div>

				<div class="flex flex-row gap-2 items-center">
					<span class="text-xs">
						Page: {currentPage}
						{perPage && rowCount ? `/ ${Math.ceil(rowCount / perPage)}` : ''}
					</span>

					{#if perPage !== undefined}
						<select class="!text-xs !w-16" bind:value={perPage}>
							<option value={25}>25</option>
							<option value={100}>100</option>
							<option value={1000}>1000</option>
						</select>
					{/if}
					<Button
						color="light"
						size="xs2"
						on:click={() => dispatch('previous')}
						disabled={currentPage === 1}
						startIcon={{ icon: ArrowLeftIcon }}
					>
						Previous
					</Button>
					{#if showNext}
						<Button
							color="light"
							size="xs2"
							on:click={() => dispatch('next')}
							endIcon={{ icon: ArrowRightIcon }}
							disabled={!hasMore}
						>
							Next
						</Button>
					{/if}
				</div>
			</div>
		{:else if shouldLoadMore}
			<div class="bg-surface border-t flex flex-row justify-center py-4 items-center gap-2">
				<Button
					color="light"
					size="xs2"
					on:click={() => dispatch('loadMore')}
					endIcon={{ icon: ArrowDownIcon }}
				>
					Load {loadMore} more
				</Button>
			</div>
		{/if}
	</List>
</div>
