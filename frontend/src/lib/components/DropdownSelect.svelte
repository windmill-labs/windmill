<script lang="ts">
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { ChevronDown } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Item } from '$lib/utils'

	interface Props {
		items?: Item[]
		extraLabel?: import('svelte').Snippet
		selected: string
		selectedDisplayName?: string
		btnClasses?: string
	}

	let { items = [], extraLabel, selected, selectedDisplayName, btnClasses }: Props = $props()

	const filteredItems = $derived(items.filter((item) => item.id !== selected))
</script>

<DropdownV2 items={filteredItems}>
	{#snippet buttonReplacement()}
		<div
			class={twMerge(
				'p-2 h-8 flex flex-row items-center gap-2 border hover:bg-surface-hover cursor-pointer rounded-md',
				btnClasses
			)}
		>
			<div class="flex flex-row items-center gap-1 pr-2 justify-between w-full">
				<span class="text-xs whitespace-nowrap">
					{selectedDisplayName ?? items.find((item) => item.id === selected)?.displayName ?? ''}
				</span>

				{@render extraLabel?.()}
			</div>
			<ChevronDown size={12} />
		</div>
	{/snippet}
</DropdownV2>
