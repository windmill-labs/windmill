<script lang="ts">
	import { toast } from '@zerodevx/svelte-toast'
	import { CheckCircle2, XCircleIcon } from 'lucide-svelte'
	import { onMount } from 'svelte'
	import Button from './common/button/Button.svelte'
	import type { ToastAction } from '$lib/toast'

	export let message: string
	export let toastId: string
	export let error: boolean = false
	export let actions: ToastAction[] = []
	export let errorMessage: string | undefined = undefined
	export let duration = 5000

	function handleClose() {
		toast.pop(toastId)
	}

	// On mount, close after 5 seconds
	onMount(() => {
		setTimeout(() => {
			toast.pop(toastId)
		}, duration)
	})
</script>

<div
	class="pointer-events-auto w-full max-w-sm overflow-hidden bg-surface shadow-lg ring-1 ring-black ring-opacity-5 border"
>
	<div class="p-2 min-h-[60px] flex flex-col">
		<div class="flex items-start w-full">
			<div class="flex-shrink-0 mt-0.5">
				{#if error}
					<XCircleIcon class="h-4 w-4 text-red-400" />
				{:else}
					<CheckCircle2 class="h-4 w-4 text-green-400" />
				{/if}
			</div>
			<div class="ml-3 flex-1 w-0">
				<p class="text-sm text-secondary break-words">{message}</p>
				{#if errorMessage}
					<p
						class="text-sm text-secondary border bg-surface-secondary p-2 w-full overflow-auto mt-2"
					>
						{errorMessage}
					</p>
				{/if}
			</div>

			<div class="ml-4 flex flex-shrink-0">
				<button
					type="button"
					on:click={handleClose}
					class="inline-flex rounded-md bg-surface-secondary text-gray-400 hover:text-tertiary focus:outline-none"
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
		<div class="mt-2 flex flex-col gap-2 h-15 items-center">
			{#each actions as action, index (index)}
				<Button
					on:click={() => {
						action.callback()
						toast.pop(toastId)
					}}
					class="text-sm !text-primary"
				>
					{action.label}
				</Button>
			{/each}
		</div>
	</div>
</div>
