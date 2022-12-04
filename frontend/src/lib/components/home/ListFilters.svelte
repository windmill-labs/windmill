<script lang="ts">
	import { classNames } from '$lib/utils'
	import { flip } from 'svelte/animate'
	import { fade } from 'svelte/transition'
	import { Badge } from '../common'

	export let filters: string[]
	export let selectedFilter: string | undefined = undefined
</script>

{#if Array.isArray(filters) && filters.length > 0}
	<div class="gap-2 w-full flex flex-wrap my-4">
		{#each filters as filter (filter)}
			<div in:fade={{ duration: 50 }} animate:flip={{ duration: 100 }}>
				<Badge
					class={classNames(
						'cursor-pointer',
						filter === selectedFilter ? 'hover:bg-blue-200' : 'hover:bg-gray-200'
					)}
					on:click={() => {
						selectedFilter = selectedFilter == filter ? undefined : filter
					}}
					color={filter === selectedFilter ? 'blue' : 'gray'}
					baseClass={filter === selectedFilter ? 'border border-blue-500' : 'border'}
				>
					{filter}
					{#if filter === selectedFilter}&cross;{/if}
				</Badge>
			</div>
		{/each}
	</div>
{/if}
