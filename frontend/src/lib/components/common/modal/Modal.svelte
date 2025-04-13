<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import Badge from '../badge/Badge.svelte'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from '../CloseButton.svelte'

	export let title: string
	export let open: boolean = false
	let c: string = ''
	export { c as class }
	export let style = ''
	export let cancelText: string | undefined = undefined
	export let kind: 'button' | 'X' = 'button'

	const dispatch = createEventDispatcher()

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			switch (event.key) {
				case 'Enter':
					event.stopPropagation()
					event.preventDefault()
					dispatch('confirmed')
					break
				case 'Escape':
					event.stopPropagation()
					event.preventDefault()
					open = false
					dispatch('canceled')
					break
			}
		}
	}
	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}
</script>

<svelte:window on:keydown={onKeyDown} />

{#if open}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
	<div
		on:click={() => (open = false)}
		transition:fadeFast|local
		class={'absolute top-0 bottom-0 left-0 right-0 z-[9999]'}
		role="dialog"
		tabindex="-1"
	>
		<div
			class={twMerge(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
			)}
		></div>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<div
					on:click|stopPropagation
					class={twMerge(
						'relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
						c,
						open
							? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
							: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
					)}
					{style}
				>
					{#if kind == 'X'}
						<div class="absolute top-4 right-4"><CloseButton on:close={() => (open = false)} /></div
						>
					{/if}
					<div class="flex">
						<div class="ml-4 text-left flex-1">
							<div class="flex flex-row items-center justify-between">
								<h3>{title}</h3>
								<slot name="settings" />
							</div>

							<div class="mt-4 text-sm text-tertiary">
								<slot />
							</div>
						</div>
					</div>
					{#if kind == 'button'}
						<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
							<slot name="actions" />
							<Button
								on:click={() => {
									dispatch('canceled')
									open = false
								}}
								color="light"
								size="sm"
							>
								<span class="inline-flex gap-2"
									>{cancelText ?? 'Cancel'}<Badge color="dark-gray">Escape</Badge></span
								>
							</Button>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
