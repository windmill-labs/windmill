<script lang="ts" generics="Item extends { label?: string; value: any }">
	import { twMerge } from 'tailwind-merge'
	import CloseButton from '../common/CloseButton.svelte'

	type Props = {
		items?: Item[]
		allowClear?: boolean
		onRemove: (item: Item) => void
		onReorder?: (oldIndex: number, newIndex: number) => void
	}
	let { items, onRemove, onReorder, allowClear = true }: Props = $props()

	let currentlyDraggingIndex: number | undefined = $state()
	let dragPos = $state<[number, number]>([0, 0])
</script>

<svelte:window
	onmousemove={(e) => {
		if (currentlyDraggingIndex === undefined) return
		dragPos = [dragPos[0] + e.movementX, dragPos[1] + e.movementY]
	}}
	onpointerup={() => {
		currentlyDraggingIndex = undefined
		dragPos = [0, 0]
	}}
/>

{#each items ?? [] as item, index}
	<li
		role="listitem"
		class={twMerge(
			allowClear ? 'pr-1' : 'pr-3',
			'pl-3 min-h-6 bg-surface-secondary rounded-full flex items-center gap-0.5',
			currentlyDraggingIndex !== undefined ? 'hover:opacity-20' : ''
		)}
		style={currentlyDraggingIndex === index
			? `transform: translate(${dragPos[0]}px, ${dragPos[1]}px); pointer-events: none;`
			: ''}
		draggable
		onpointerdown={(e) => {
			e.stopPropagation()
			dragPos = [0, 0]
			currentlyDraggingIndex = index
		}}
		onpointerup={(e) => {
			if (currentlyDraggingIndex !== undefined) {
				e.stopPropagation()
				onReorder?.(currentlyDraggingIndex, index)
			}
			currentlyDraggingIndex = undefined
			dragPos = [0, 0]
		}}
	>
		<span class="text-sm select-none">{item.label || item.value}</span>
		{#if allowClear}
			<CloseButton
				class="text-tertiary"
				small
				on:close={(e) => (onRemove(item), e.stopPropagation())}
			/>
		{/if}
	</li>
{/each}
