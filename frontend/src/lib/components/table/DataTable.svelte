<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Button from '../common/button/Button.svelte'
	import { ArrowLeftIcon, ArrowRightIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let paginated: boolean = false
	export let currentPage: number = 1
	export let showNext: boolean = true
	export let loadMore: number = 0
	export let shouldLoadMore: boolean = false
	export let rounded: boolean = true

	const dispatch = createEventDispatcher()
</script>

<div class={twMerge('border  table-scroll overflow-hidden', rounded ? 'rounded-md' : '')}>
	<table class="min-w-full block divide-y">
		<slot />
	</table>
	{#if paginated}
		<div class="bg-surface border-t flex flex-row justify-end p-1 items-center gap-2">
			<span class="text-xs">Page: {currentPage}</span>
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
		<div class="bg-surface border-t flex flex-row justify-end p-1 items-center gap-2">
			<Button color="light" size="xs2" on:click={() => dispatch('loadMore')}>
				<div class="flex flex-row gap-1 items-center">
					Load {loadMore} more
					<ArrowRightIcon size={16} />
				</div>
			</Button>
		</div>
	{/if}
</div>

<style>
	.table-scroll {
		max-height: 60vh;
		overflow-y: auto;
	}
</style>
