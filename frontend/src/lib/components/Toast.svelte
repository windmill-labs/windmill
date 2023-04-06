<script lang="ts">
	import { toast } from '@zerodevx/svelte-toast'
	import { CheckCircle2, XCircleIcon } from 'lucide-svelte'
	import { onMount } from 'svelte'

	export let message: string
	export let toastId: string
	export let error: boolean = false

	function handleClose() {
		toast.pop(toastId)
	}

	// On mount, close after 5 seconds
	onMount(() => {
		setTimeout(() => {
			toast.pop(toastId)
		}, 5000)
	})
</script>

<div
	class="pointer-events-auto w-full max-w-sm overflow-hidden bg-white shadow-lg ring-1 ring-black ring-opacity-5 border"
>
	<div class="p-4">
		<div class="flex items-start">
			<div class="flex-shrink-0">
				{#if error}
					<XCircleIcon class="h-6 w-6 text-red-400" />
				{:else}
					<CheckCircle2 class="h-6 w-6 text-green-400" />
				{/if}
			</div>
			<div class="ml-3 w-0 flex-1 pt-0.5">
				<p class="text-sm font-medium text-gray-900">{error ? 'Error' : 'Success'}</p>
				<p class="mt-1 text-sm text-gray-500">{message}</p>
			</div>
			<div class="ml-4 flex flex-shrink-0">
				<button
					type="button"
					on:click={handleClose}
					class="inline-flex rounded-md bg-white text-gray-400 hover:text-gray-500 focus:outline-none"
				>
					<span class="sr-only">Close</span>
					<svg class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
						<path
							d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z"
						/>
					</svg>
				</button>
			</div>
		</div>
	</div>
</div>
