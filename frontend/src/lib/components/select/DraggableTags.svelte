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
			allowClear ? 'pr-0' : 'pr-2',
			'pl-2 min-h-6 border bg-surface rounded-full flex items-center',
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
		<span class="text-xs select-none">{item.label || item.value}</span>
		{#if allowClear}
			<CloseButton
				class="text-hint bg-transparent border-none"
				small
				on:close={(e) => (onRemove(item), e.stopPropagation())}
			/>
		{/if}
	</li>
{/each}
