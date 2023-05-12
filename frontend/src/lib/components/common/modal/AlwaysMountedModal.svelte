<script lang="ts">
	import { onDestroy } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { Badge } from '..'
	import Button from '../button/Button.svelte'

	export let title: string
	export let style: string = ''

	let isVisible: boolean = false
	let modalDiv: HTMLDivElement

	function onKeyDown(event: KeyboardEvent) {
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

	export function open() {
		isVisible = true
		window.addEventListener('keydown', onKeyDown)
		document.body.style.overflow = 'hidden'
		document.body.appendChild(modalDiv)
	}

	export function close() {
		isVisible = false
		window.removeEventListener('keydown', onKeyDown)
		document.body.style.overflow = ''
		document.body.removeChild(modalDiv)
	}

	onDestroy(() => {
		window.removeEventListener('keydown', onKeyDown)
	})
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div id="topModal" class:visible={isVisible} bind:this={modalDiv} on:click={() => close()}>
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
