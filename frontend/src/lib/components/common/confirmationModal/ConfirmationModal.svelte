<script lang="ts">
	import { classNames } from '$lib/utils'
	import { faWarning } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'

	export let title: string
	export let confirmationText: string

	export let open: boolean = false

	const dispatch = createEventDispatcher()
</script>

{#if open}
	<div transition:fade={{ duration: 100 }} class={'relative  z-50'} role="dialog">
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
						'relative transform overflow-hidden rounded-lg bg-white px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
						open
							? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
							: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
					)}
				>
					<div class="flex">
						<div class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100">
							<Icon data={faWarning} class="text-red-500" />
						</div>
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
						<Button on:click={() => dispatch('confirmed')} color="red" size="sm">
							{confirmationText}
						</Button>
						<Button on:click={() => dispatch('canceled')} color="light" size="sm">Cancel</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
