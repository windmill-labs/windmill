<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import TopLevelNode from './TopLevelNode.svelte'

	export let label: string
	export let selected = false
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

<svelte:window on:keydown={handleKeydown} />

<TopLevelNode {label} {selected} returnIcon onSelect={click} />
