<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import Badge from '../badge/Badge.svelte'
	import { AlertTriangle } from 'lucide-svelte'

	export let title: string
	export let confirmationText: string

	export let open: boolean = false

	const dispatch = createEventDispatcher()

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			event.stopPropagation()
			event.preventDefault()
			switch (event.key) {
				case 'Enter':
					dispatch('confirmed')
					break
				case 'Escape':
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
	<div
		transition:fadeFast|local
		class={'absolute top-0 bottom-0 left-0 right-0 z-[5000]'}
		role="dialog"
	>
		<div
			class={classNames(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
			)}
		/>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class={classNames(
						'relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
						open
							? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
							: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
					)}
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100 dark:bg-red-800/50"
						>
							<AlertTriangle class="text-red-500 dark:text-red-400" />
						</div>
						<div class="ml-4 text-left flex-1">
							<h3 class="text-lg font-medium text-primary">
								{title}
							</h3>
							<div class="mt-2 text-sm text-secondary">
								<slot />
							</div>
						</div>
					</div>
					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						<Button on:click={() => dispatch('confirmed')} color="red" size="sm">
							<span>{confirmationText} <Badge>Enter</Badge></span>
						</Button>
						<Button on:click={() => dispatch('canceled')} color="light" size="sm">
							<span>Cancel <Badge color="dark-gray">Escape</Badge></span>
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
