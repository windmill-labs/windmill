<script lang="ts" generics="Item extends { label?: string; value: any }">
	import CloseButton from '../common/CloseButton.svelte'

	type Props = {
		items?: Item[]
		onRemove: (item: Item) => void
		onReorder?: (item: Item, newIndex: number) => void
	}
	let { items, onRemove, onReorder }: Props = $props()

	let currentlyDragging: Item | undefined
</script>

<svelte:window onpointerup={(e) => (currentlyDragging = undefined)} />

{#each items ?? [] as item, index}
	<div
		class={'pl-3 pr-1 bg-surface-secondary rounded-full flex items-center gap-0.5'}
		draggable
		onpointerdown={(e) => {
			e.stopPropagation()
			currentlyDragging = item
		}}
		onpointerup={(e) => {
			e.stopPropagation()
			if (currentlyDragging) onReorder?.(currentlyDragging, index)
			currentlyDragging = undefined
		}}
	>
		<span class="text-sm">{item.label || item.value}</span>
		<CloseButton
			class="text-tertiary"
			small
			on:close={(e) => (onRemove(item), e.stopPropagation())}
		/>
	</div>
{/each}
