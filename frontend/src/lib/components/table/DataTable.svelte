<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Button from '../common/button/Button.svelte'
	import { ArrowLeftIcon, ArrowRightIcon } from 'lucide-svelte'

	export let paginated: boolean = false
	export let currentPage: number = 1
	export let showNext: boolean = true
	export let loadMore: number = 0
	export let shouldLoadMore: boolean = false

	const dispatch = createEventDispatcher()
</script>

<div class="rounded-md border overflow-hidden">
	<table class="min-w-full divide-y">
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
