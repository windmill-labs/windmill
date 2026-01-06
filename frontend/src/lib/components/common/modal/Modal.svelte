<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from '../CloseButton.svelte'

	interface Props {
		title: string
		open?: boolean
		class?: string
		style?: string
		cancelText?: string | undefined
		kind?: 'button' | 'X'
		settings?: import('svelte').Snippet
		children?: import('svelte').Snippet
		actions?: import('svelte').Snippet
	}

	let {
		title,
		open = $bindable(false),
		class: c = '',
		style = '',
		cancelText = undefined,
		kind = 'button',
		settings,
		children,
		actions
	}: Props = $props()

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

<svelte:window onkeydowncapture={onKeyDown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		onclick={() => (open = false)}
		transition:fadeFast|local
		class={'fixed top-0 bottom-0 left-0 right-0 z-[9999]'}
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
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					onclick={stopPropagation(bubble('click'))}
					class={twMerge(
						'relative transform overflow-hidden rounded-md bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
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
								<h3 class="text-emphasis text-lg font-semibold">{title}</h3>
								{@render settings?.()}
							</div>

							<div class="mt-4 text-sm text-primary">
								{@render children?.()}
							</div>
						</div>
					</div>
					{#if kind == 'button'}
						<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
							{@render actions?.()}
							<Button
								on:click={() => {
									dispatch('canceled')
									open = false
								}}
								color="light"
								size="sm"
							>
								{cancelText ?? 'Cancel'}
							</Button>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
