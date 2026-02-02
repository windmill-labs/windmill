<script lang="ts" module>
	type ToastState = { hover: boolean; elapsed: number; duration: number }
	const toastStates: Record<string, ToastState> = $state({})

	let lastTime = 0
	let isLoopRunning = false
	function update(time: number) {
		isLoopRunning = true
		const delta = time - lastTime

		let hover = Object.values(toastStates).some((state) => state.hover)
		for (const toastId in toastStates) {
			const state = toastStates[toastId]
			if (hover) continue
			if (state.elapsed >= state.duration) {
				delete toastStates[toastId]
				continue
			}
			state.elapsed += delta
		}
		lastTime = time

		if (Object.values(toastStates).length > 0) {
			requestAnimationFrame(update)
		} else {
			isLoopRunning = false
		}
	}

	function registerToast(toastId: string, duration: number) {
		toastStates[toastId] = { hover: false, elapsed: 0, duration }
		if (!isLoopRunning) {
			requestAnimationFrame((time) => {
				lastTime = time
				update(time)
			})
		}
	}

	setInterval(() => {
		sendUserToast('Testing new toasts ! Success.')
	}, 3000)
</script>

<script lang="ts">
	import { toast } from '@zerodevx/svelte-toast'
	import { CheckCircle2, XCircleIcon } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { sendUserToast, type ToastAction } from '$lib/toast'
	import { processMessage } from './toast'
	import { onDestroy, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { classes } from '$lib/components/common/alert/model'

	interface Props {
		message: string
		toastId: string
		error?: boolean
		actions?: ToastAction[]
		errorMessage?: string | undefined
		duration?: number
	}

	let {
		message,
		toastId,
		error = false,
		actions = [],
		errorMessage = undefined,
		duration = 5000
	}: Props = $props()

	function handleClose() {
		toast.pop(toastId)
	}

	$effect.pre(() => {
		untrack(() => registerToast(toastId, duration))
	})
	onDestroy(() => {
		delete toastStates[toastId]
	})
	let state = $derived.by(() => toastStates[toastId] as ToastState | undefined)
	$effect(() => {
		if (!state) {
			toast.pop(toastId)
		}
	})

	let color = error ? classes.error : classes.success
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={twMerge(
		'pointer-events-auto w-full overflow-hidden rounded-md relative flex items-center bg-surface',
		error ? 'toast-error' : 'toast-success'
	)}
	onmouseenter={() => state && (state.hover = true)}
	onmouseleave={() => state && (state.hover = false)}
>
	<div
		class="flex items-center h-full w-full min-h-10 rounded-md px-2 py-1 {color.descriptionClass} {color.bgClass}"
	>
		<div class="flex-shrink-0 mt-0.5">
			{#if error}
				<XCircleIcon class="h-4 w-4 {color.iconClass}" />
			{:else}
				<CheckCircle2 class="h-4 w-4 {color.iconClass}" />
			{/if}
		</div>
		<div class="ml-3 flex-1 w-0">
			<p class="text-xs break-words">
				{@html processMessage(message)}
			</p>
			{#if errorMessage}
				<p class="text-xs {color.descriptionClass} w-full overflow-auto mt-2">
					{errorMessage}
				</p>
			{/if}
		</div>

		<div class="flex gap-2 items-center">
			{#each actions as action, index (index)}
				<Button
					variant={action.buttonType ?? 'subtle'}
					unifiedSize="sm"
					onClick={() => {
						action.callback()
						toast.pop(toastId)
					}}
				>
					{action.label}
				</Button>
			{/each}
		</div>

		<div class="ml-4 flex flex-shrink-0">
			<button
				type="button"
				onclick={handleClose}
				class="inline-flex rounded-md hover:text-primary focus:outline-none"
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
	<!-- Duration indicator -->
	<div
		class="h-[1px] absolute bottom-0 transition-colors bg-current {classes.success[
			'iconClass'
		]} opacity-40"
		style="width: {Math.max(0, 1 - (state?.elapsed ?? duration) / duration) * 100}%"
	>
	</div>
</div>
