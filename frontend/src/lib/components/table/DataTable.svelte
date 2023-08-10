<script context="module" lang="ts">
	export type DatatableContext = {
		size: 'sm' | 'md' | 'lg'
	}
</script>

<script lang="ts">
	import { createEventDispatcher, setContext } from 'svelte'
	import Button from '../common/button/Button.svelte'
	import { ArrowDownIcon, ArrowLeftIcon, ArrowRightIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let paginated: boolean = false
	export let currentPage: number = 1
	export let showNext: boolean = true
	export let loadMore: number = 0
	export let shouldLoadMore: boolean = false
	export let rounded: boolean = true
	export let size: 'sm' | 'md' | 'lg' = 'md'
	export let perPage: number | undefined = undefined
	export let shouldHidePagination: boolean = false

	const dispatch = createEventDispatcher()

	setContext<DatatableContext>('datatable', {
		size
	})
</script>

<div class={twMerge('border h-full overflow-auto', rounded ? 'rounded-md' : '')}>
	<table class={twMerge('min-w-full divide-y')}>
		<slot />
	</table>
	{#if paginated && !shouldHidePagination}
		<div
			class="bg-surface border-t flex flex-row justify-end p-1 items-center gap-2 sticky bottom-0"
		>
			<span class="text-xs">Page: {currentPage}</span>

			{#if perPage !== undefined}
				<select class="text-xs !w-16" bind:value={perPage}>
					<option value={25}>25</option>
					<option value={50}>50</option>
					<option value={100}>100</option>
				</select>
			{/if}
			<Button
				color="light"
				size="xs2"
				on:click={() => dispatch('previous')}
				disabled={currentPage === 1}
			>
				<div class="flex flex-row gap-1 items-center">
					<ArrowLeftIcon size={16} />

					Previous
				</div>
			</Button>
			{#if showNext}
				<Button color="light" size="xs2" on:click={() => dispatch('next')}>
					<div class="flex flex-row gap-1 items-center">
						Next
						<ArrowRightIcon size={16} />
					</div>
				</Button>
			{/if}
		</div>
	{:else if shouldLoadMore}
		<div class="bg-surface border-t flex flex-row justify-center py-4 items-center gap-2">
			<Button color="light" size="xs2" on:click={() => dispatch('loadMore')}>
				<div class="flex flex-row gap-1 items-center">
					Load {loadMore} more
					<ArrowDownIcon size={16} />
				</div>
			</Button>
		</div>
	{/if}
</div>
