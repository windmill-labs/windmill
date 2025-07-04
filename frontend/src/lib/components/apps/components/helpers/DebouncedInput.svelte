<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { onMount } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	// Using 'any' so 'type="number"' can be passed to the input

	interface Props {
		placeholder?: string
		// which should return a number
		value: any
		debounceDelay?: number
		class?: string | undefined
		[key: string]: any
	}

	let {
		placeholder = 'Search...',
		value = $bindable(),
		debounceDelay = 100,
		class: parentClass = undefined,
		...rest
	}: Props = $props()

	let timer: NodeJS.Timeout
	let inputElement: HTMLInputElement | null = $state(null)

	function debounce(event: KeyboardEvent): void {
		clearTimeout(timer)

		timer = setTimeout(() => {
			const target = event.target as HTMLInputElement
			value = target.value
		}, debounceDelay)
	}

	onMount(() => {
		if (inputElement && value !== undefined) {
			inputElement.value = value
		}
	})
</script>

<input
	bind:this={inputElement}
	{placeholder}
	onpointerdown={stopPropagation(bubble('pointerdown'))}
	onkeyup={debounce}
	onkeydown={stopPropagation(bubble('keydown'))}
	class={twMerge(parentClass, 'mb-1 h-8 !rounded-md !shadow-none')}
	{...rest}
/>
