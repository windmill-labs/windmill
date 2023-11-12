<script lang="ts" context="module">
	let openedDrawers: string[] = []
</script>

<script lang="ts">
	import { onMount } from 'svelte'
	import { createEventDispatcher } from 'svelte'
	import { BROWSER } from 'esm-env'

	export let open = false
	export let duration = 0.3
	export let placement = 'right'
	export let size = '600px'
	export let alwaysOpen = false

	let id = (Math.random() + 1).toString(36).substring(10)

	$: durationMs = duration * 1000

	export function toggleDrawer() {
		open = !open
		if (open) {
			openedDrawers.push(id)
		} else {
			openedDrawers = openedDrawers.filter((x) => x != id)
		}
	}

	export function openDrawer() {
		openedDrawers.push(id)
		open = true
	}
	export function closeDrawer() {
		open = false
		openedDrawers = openedDrawers.filter((x) => x != id)

		setTimeout(() => {
			dispatch('afterClose')
		}, durationMs)
	}

	export function isOpen() {
		return open
	}

	let mounted = false
	const dispatch = createEventDispatcher()

	$: style = `--duration: ${duration}s; --size: ${size};`

	function scrollLock(open: boolean) {
		if (BROWSER) {
			const body = document.querySelector('body')

			if (mounted && body) {
				body.style.overflowY = open ? 'hidden' : 'auto'
			}
		}
	}

	$: scrollLock(open)

	function handleClickAway() {
		dispatch('clickAway')
		open = false
	}

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			switch (event.key) {
				case 'Escape':
					if (id == openedDrawers[openedDrawers.length - 1] || openedDrawers.length == 0) {
						openedDrawers.pop()
						event.preventDefault()
						event.stopPropagation()
						event.stopImmediatePropagation()
						open = false
						break
					}
			}
		}
	}

	$: open ? dispatch('open') : dispatch('close')

	let timeout = true
	$: !open ? setTimeout(() => (timeout = true), durationMs) : (timeout = false)
	onMount(() => {
		mounted = true
	})
</script>

<svelte:window on:keydown={onKeyDown} />

<aside
	class="drawer {$$props.class ?? ''} {$$props.positionClass ?? ''}"
	class:open
	class:close={!open && timeout}
	{style}
>
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="overlay {$$props.positionClass ?? ''}" on:click={handleClickAway} />
	<div class="panel {placement} {$$props.positionClass}" class:size>
		{#if open || !timeout || alwaysOpen}
			<slot {open} />
		{/if}
	</div>
</aside>

<style>
	.drawer {
		position: fixed;
		top: 0;
		left: 0;
		height: 100%;
		width: 100%;
		z-index: -1;
		transition: z-index var(--duration) step-end;
		overflow: clip;
		pointer-events: none;
	}

	.drawer.open {
		height: 100%;
		width: 100%;
		z-index: var(--zIndex, 1002);
		transition: z-index var(--duration) step-start;
		pointer-events: auto;
	}

	.overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0, 0, 0, 0.5);
		opacity: 0;
		z-index: 2;
		transition: opacity var(--duration) ease;
	}

	.drawer.open > .overlay {
		opacity: 1;
	}

	.drawer.close > .panel {
		height: 0;
		overflow: hidden;
	}

	.panel {
		position: fixed;
		width: 100%;
		@apply bg-surface;
		z-index: 3;
		transition: transform var(--duration) ease;
		height: 100%;
	}

	.panel.left {
		left: 0;
		transform: translate(-100%, 0);
	}

	.panel.right {
		right: 0;
		transform: translate(100%, 0);
	}

	.panel.top {
		top: 0;
		transform: translate(0, -100%);
	}

	.panel.bottom {
		bottom: 0;
		transform: translate(0, 100%);
	}

	.panel.left.size,
	.panel.right.size {
		max-width: var(--size);
	}

	.panel.top.size,
	.panel.bottom.size {
		max-height: var(--size);
	}

	.drawer.open > .panel {
		transform: translate(0, 0);
	}
</style>
