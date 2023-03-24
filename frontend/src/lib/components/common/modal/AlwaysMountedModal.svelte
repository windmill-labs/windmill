<script context="module" lang="ts">
	let onTop: HTMLDivElement
	const modals = {}

	export function getModal(id = '') {
		return modals[id]
	}
</script>

<script lang="ts">
	import { onDestroy } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { Badge } from '..'
	import Button from '../button/Button.svelte'

	let topDiv: HTMLDivElement
	let visible: boolean = false
	let prevOnTop: HTMLDivElement

	export let title: string
	export let style = ''

	export let id = ''

	function onKeyDown(event: KeyboardEvent) {
		if (onTop == topDiv) {
			switch (event.key) {
				case 'Enter':
					event.stopPropagation()
					event.preventDefault()
					break
				case 'Escape':
					event.stopPropagation()
					event.preventDefault()
					close()
					break
			}
		}
	}

	function open() {
		if (visible) {
			return
		}
		prevOnTop = onTop
		onTop = topDiv
		window.addEventListener('keydown', onKeyDown)
		document.body.style.overflow = 'hidden'
		visible = true
		document.body.appendChild(topDiv)
	}

	function close() {
		if (!visible) {
			return
		}
		window.removeEventListener('keydown', onKeyDown)

		onTop = prevOnTop
		if (onTop == null) {
			document.body.style.overflow = ''
		}
		visible = false
	}

	modals[id] = { open, close }

	onDestroy(() => {
		delete modals[id]
		window.removeEventListener('keydown', onKeyDown)
	})
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div id="topModal" class:visible bind:this={topDiv} on:click={() => close()}>
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="relative bg-white p-4 rounded-md" on:click|stopPropagation={() => {}}>
		<div class={twMerge('max-w-screen-lg max-h-screen-80 overflow-auto', $$props.class)} {style}>
			<div class="flex">
				<div class="ml-4 text-left flex-1">
					<h3 class="text-lg font-medium text-gray-900">
						{title}
					</h3>
					<div class="mt-2 text-sm text-gray-500">
						<slot />
					</div>
				</div>
			</div>
			<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
				<Button
					on:click={() => {
						close()
					}}
					color="light"
					size="sm"
				>
					<span class="gap-2">Cancel <Badge color="dark-gray">Escape</Badge></span>
				</Button>
			</div>
		</div>
	</div>
</div>

<style>
	#topModal {
		visibility: hidden;
		z-index: 9999;
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: #4448;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.visible {
		visibility: visible !important;
	}
</style>
