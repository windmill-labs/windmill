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
			const st = toastStates[toastId]
			if (hover) continue
			if (st.elapsed >= st.duration) {
				delete toastStates[toastId]
				continue
			}
			st.elapsed += delta
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
</script>

<script lang="ts">
	import { toast } from '@zerodevx/svelte-toast'
	import Button from './common/button/Button.svelte'
	import { type ToastAction, type ToastType } from '$lib/toast'
	import { processMessage } from './toast'
	import { onDestroy, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { classes, icons } from '$lib/components/common/alert/model'

	interface Props {
		message: string
		toastId: string
		type?: ToastType
		actions?: ToastAction[]
		errorMessage?: string | undefined
		duration?: number
	}

	let {
		message,
		toastId,
		type = 'success',
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
	let _state = $derived.by(() => toastStates[toastId] as ToastState | undefined)
	$effect(() => {
		if (!_state) {
			toast.pop(toastId)
		}
	})

	let color = classes[type]

	let containerClass = {
		success: 'toast-success',
		error: 'toast-error',
		info: 'toast-info',
		warning: 'toast-warning'
	}[type]

	let Icon = $derived(icons[type])

	let showMore = $state(false)
	const MAX_MSG_LEN = 160
	let isLongMessage = $derived((message ?? '').length > MAX_MSG_LEN)
	let displayMessage = $derived(
		isLongMessage && !showMore ? (message ?? '').slice(0, MAX_MSG_LEN) + '... ' : (message ?? '')
	)
	// let hover = $derived(Object.values(toastStates).some((state) => state.hover))
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={twMerge(
		'pointer-events-auto w-full overflow-hidden rounded-md relative flex items-center bg-surface',
		containerClass
	)}
	onmouseenter={() => _state && (_state.hover = true)}
	onmouseleave={() => _state && (_state.hover = false)}
>
	<div
		class="flex items-center h-full w-full min-h-10 rounded-md px-2 py-1 {color.descriptionClass} {color.bgClass}"
	>
		<div class="flex-shrink-0 mt-0.5">
			<Icon class="h-4 w-4 {color.iconClass}" />
		</div>
		<div class="ml-3 flex-1 w-0">
			<p class="text-xs break-words">
				{@html processMessage(displayMessage)}
				{#if isLongMessage && !showMore}
					<button
						type="button"
						class="ml-1 {color.descriptionClass} font-medium hover:underline focus:outline-none"
						onclick={() => (showMore = true)}
					>
						Show more
					</button>
				{/if}
			</p>
			{#if errorMessage}
				<p class="text-xs {color.descriptionClass} w-full overflow-auto mt-2">
					{errorMessage}
				</p>
			{/if}
		</div>

		<div class="flex justify-center ml-2">
			{#each actions as action, index (index)}
				<Button
					variant={action.buttonType ?? 'subtle'}
					unifiedSize="sm"
					onClick={() => {
						action.callback()
						toast.pop(toastId)
					}}
					btnClasses="{color.descriptionClass} font-medium"
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
		class="h-[1px] absolute bottom-0 transition-colors bg-current {color.iconClass} opacity-60"
		style="width: {Math.max(0, 1 - (_state?.elapsed ?? duration) / duration) * 100}%"
	>
	</div>
</div>
