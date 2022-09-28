<script lang="ts">
	import { classNames } from '$lib/utils'
	import { faWarning } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'

	export let title: string
	export let description: string
	export let confirmationText: string

	export let open: boolean = false

	const dispatch = createEventDispatcher()
</script>

<div class={classNames('relative', open ? 'z-50' : '-z-50')} role="dialog">
	<div
		class={classNames(
			'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
			open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
		)}
	/>

	<div class="fixed inset-0 z-10 overflow-y-auto">
		<div class="flex min-h-full items-end justify-center p-4 text-center sm:items-center sm:p-0">
			<div
				class={classNames(
					'relative transform overflow-hidden rounded-lg bg-white px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
					open
						? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
						: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
				)}
			>
				<div class="sm:flex sm:items-start">
					<div
						class="mx-auto flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-red-100 sm:mx-0 sm:h-10 sm:w-10"
					>
						<Icon data={faWarning} class="text-red-500" />
					</div>
					<div class="mt-3 text-center sm:mt-0 sm:ml-4 sm:text-left">
						<h3 class="text-lg font-medium leading-6 text-gray-900" id="modal-title">
							{title}
						</h3>
						<div class="mt-2">
							<p class="text-sm text-gray-500">
								{description}
							</p>
						</div>
					</div>
				</div>
				<div class="mt-5 sm:mt-4 sm:flex sm:flex-row-reverse">
					<button
						type="button"
						class="inline-flex w-full justify-center rounded-md border border-transparent bg-red-600 px-4 py-2 text-base font-medium text-white shadow-sm hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 sm:ml-3 sm:w-auto sm:text-sm"
						on:click={() => dispatch('confirmed')}
					>
						{confirmationText}
					</button>
					<button
						type="button"
						on:click={() => dispatch('canceled')}
						class="mt-3 inline-flex w-full justify-center rounded-md border border-gray-300 bg-white px-4 py-2 text-base font-medium text-gray-700 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 sm:mt-0 sm:w-auto sm:text-sm"
					>
						Cancel
					</button>
				</div>
			</div>
		</div>
	</div>
</div>
