<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Badge } from '$lib/components/common'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import type { HubFlow } from '$lib/stores'

	export let item: HubFlow & { marked?: string }

	const dispatch = createEventDispatcher()
</script>

<button
	class="p-4 gap-4 flex flex-row grow justify-between hover:bg-gray-50 bg-white transition-all items-center rounded-md"
	on:click={() => dispatch('pick', item)}
>
	<div class="flex items-center gap-4">
		<RowIcon kind="flow" />

		<div class="w-full text-left font-normal">
			<div class="text-gray-900 flex-wrap text-md font-semibold mb-1">
				{#if item.marked}
					{@html item.marked ?? ''}
				{:else}
					{item.summary ?? ''}
				{/if}
			</div>
		</div>
	</div>
	<div class="min-w-1/3 gap-2 flex flex-wrap justify-end">
		{#each item.apps as app}
			<Badge color="gray" baseClass="border">{app}</Badge>
		{/each}
	</div>
</button>
