<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import TopLevelNode from './TopLevelNode.svelte'

	interface Props {
		label: string
		selected?: boolean
		onHover?: () => void
	}

	let { label, selected = false, onHover = undefined }: Props = $props()
	const dispatch = createEventDispatcher()
	function handleKeydown(event: KeyboardEvent & { currentTarget: EventTarget & Window }) {
		if (selected && event.key === 'Enter') {
			event.preventDefault()
			click()
		}
	}

	function click() {
		dispatch('click')
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<TopLevelNode {label} {selected} returnIcon neutral onSelect={click} {onHover} />
