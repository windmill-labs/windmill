<script lang="ts">
	import { clickOutside } from '$lib/utils'
	import { fly } from 'svelte/transition'
	import Portal from './Portal.svelte'

	let isOpen = $state(false)
	let mousePos = $state({ x: 0, y: 0 })
	export function open(e: MouseEvent) {
		e.preventDefault()
		isOpen = true
		mousePos = { x: e.clientX, y: e.clientY }
		console.log('Opening popover at', mousePos)
	}
	export function close() {
		isOpen = false
	}
</script>

<Portal>
	{#if isOpen}
		<div
			transition:fly={{ x: 0, y: -10, duration: 200 }}
			use:clickOutside={{
				onClickOutside: (e) => {
					isOpen = false
					e.preventDefault()
					e.stopPropagation()
				}
			}}
			class="absolute left-0 top-0 z-[9999] w-fit"
			style="transform: translate({mousePos.x}px, {mousePos.y}px)"
		>
		</div>
	{/if}
</Portal>
