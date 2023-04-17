<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Badge } from '$lib/components/common'
	import { capitalize, classNames } from '$lib/utils'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import type { HubScript } from '$lib/stores'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'
	export let item: HubScript & { marked?: string }

	const dispatch = createEventDispatcher()
</script>

<button
	class="p-4 gap-4 flex flex-row grow hover:bg-gray-50 bg-white transition-all items-center rounded-md"
	on:click={() => dispatch('pick', item)}
>
	<div class="flex items-center gap-4">
		<div
			class={classNames(
				'rounded-md p-1 flex justify-center items-center border',
				'bg-gray-50 border-gray-200'
			)}
		>
			<svelte:component this={APP_TO_ICON_COMPONENT[item['app']]} height={18} width={18} />
		</div>

		<div class="w-full text-left font-normal">
			<div class="text-gray-900 flex-wrap text-md font-semibold mb-1">
				{#if item.marked}
					{@html item.marked ?? ''}
				{:else}
					{item.summary ?? ''}
				{/if}
			</div>
			<div class="text-gray-600 text-xs">
				{item.path}
			</div>
		</div>
	</div>
	{#if kind !== 'script'}
		<Badge color="gray" baseClass="border">{capitalize(kind)}</Badge>
	{/if}
</button>
