<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Ellipsis } from 'lucide-svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Runnable } from '../apps/inputType'

	interface Props {
		id: string
		runnable: Runnable
		isSelected: boolean
		onSelect: () => void
		onDelete: () => void
	}

	let { id, runnable, isSelected, onSelect, onDelete }: Props = $props()

	let dropdownOpen = $state(false)
</script>

<button
	{id}
	class="relative w-full gap-1 flex items-center h-6 rounded-md px-1 group
	{isSelected ? 'bg-surface-accent-selected text-accent' : 'hover:bg-surface-hover'}"
	onclick={onSelect}
>
	<Badge color="indigo" class={isSelected ? 'bg-surface-tertiary' : ''}>{id}</Badge>
	<span class="text-xs truncate font-normal pr-6">{runnable?.name}</span>
	<DropdownV2
		items={[
			{
				displayName: 'Delete',
				action: onDelete,
				type: 'delete'
			}
		]}
		class={twMerge(
			'absolute -translate-y-1/2 top-1/2 right-1 hidden group-hover:block',
			dropdownOpen ? 'block' : ''
		)}
		bind:open={dropdownOpen}
	>
		{#snippet buttonReplacement()}
			<Button
				iconOnly
				unifiedSize="xs"
				variant="subtle"
				startIcon={{ icon: Ellipsis }}
				nonCaptureEvent
			/>
		{/snippet}
	</DropdownV2>
</button>
