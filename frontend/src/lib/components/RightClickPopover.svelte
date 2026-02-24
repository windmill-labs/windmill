<script lang="ts">
	import { clickOutside } from '$lib/utils'
	import { fly } from 'svelte/transition'
	import Portal from './Portal.svelte'
	import type { Snippet } from 'svelte'

	type Props = {
		children: Snippet
	}

	const { children }: Props = $props()

	let _isOpen = $state(false)
	let mousePos = $state({ x: 0, y: 0 })
	export function open(e: MouseEvent) {
		e.preventDefault()
		_isOpen = true
		mousePos = { x: e.clientX, y: e.clientY }
	}
	export function close() {
		_isOpen = false
	}
	export function isOpen() {
		return _isOpen
	}
</script>

<Portal>
	{#if _isOpen}
		<div
			in:fly={{ x: 0, y: -10, duration: 120 }}
			use:clickOutside={{
				onClickOutside: (e) => {
					_isOpen = false
					e.preventDefault()
					e.stopPropagation()
				}
			}}
			class="absolute left-0 top-0 z-[9999] w-fit"
			style="transform: translate({mousePos.x + 2}px, {mousePos.y + 2}px)"
		>
			{@render children()}
		</div>
	{/if}
</Portal>
