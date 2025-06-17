<script lang="ts" generics="Item extends { label?: string; value: any }">
	import CloseButton from '../common/CloseButton.svelte'

	type Props = {
		items?: Item[]
		onRemove: (item: Item) => void
		onReorder?: (item: Item, newIndex: number) => void
	}
	let { items, onRemove, onReorder }: Props = $props()

	let currentlyDragging: { item: Item; index: number } | undefined = $state()
	let dragPos = $state<[number, number]>([0, 0])
</script>

<svelte:window
	onmousemove={(e) => {
		if (!currentlyDragging) return
		dragPos = [dragPos[0] + e.movementX, dragPos[1] + e.movementY]
	}}
	onpointerup={() => ((currentlyDragging = undefined), (dragPos = [0, 0]))}
/>

{#each items ?? [] as item, index}
	<div
		class={'pl-3 pr-1 bg-surface-secondary rounded-full flex items-center gap-0.5'}
		style={currentlyDragging?.index === index
			? `transform: translate(${dragPos[0]}px, ${dragPos[1]}px); pointer-events: none;`
			: ''}
		draggable
		onpointerdown={(e) => {
			e.stopPropagation()
			dragPos = [0, 0]
			currentlyDragging = { item, index }
		}}
		onpointerup={(e) => {
			e.stopPropagation()
			if (currentlyDragging) onReorder?.(currentlyDragging.item, index)
			currentlyDragging = undefined
			dragPos = [0, 0]
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
