<script lang="ts">
	import { onMount } from 'svelte'
	import { createEventDispatcher } from 'svelte'
	import { browser } from '$app/environment'

	export let open = false
	export let duration = 0.3
	export let placement = 'right'
	export let size = '600px'
	export let alwaysOpen = false

	$: durationMs = duration * 1000

	export function toggleDrawer() {
		open = !open
	}

	export function openDrawer() {
		open = true
	}
	export function closeDrawer() {
		open = false
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
		if (browser) {
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
					event.preventDefault()
					event.stopPropagation()
					event.stopImmediatePropagation()
					open = false
					break
			}
		}
	}

	$: open ? dispatch('open') : dispatch('close')

	let timeout = true
	$: !open ? setTimeout(() => (timeout = true), durationMs) : (timeout = false)
	onMount(() => {
		mounted = true
		scrollLock(open)
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
	}

	.drawer.open {
		height: 100%;
		width: 100%;
		z-index: 1002;
		transition: z-index var(--duration) step-start;
	}

	.overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(100, 100, 100, 0.5);
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
		background: white;
		z-index: 3;
		transition: transform var(--duration) ease;
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
