<script lang="ts">
	import { onMount } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let placeholder: string = 'Search...'
	// Using 'any' so 'type="number"' can be passed to the input
	// which should return a number
	export let value: any
	export let debounceDelay: number = 100

	let parentClass: string | undefined = undefined
	export { parentClass as class }

	let timer: NodeJS.Timeout
	let inputElement: HTMLInputElement | null = null

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
	on:pointerdown|stopPropagation
	on:keyup={debounce}
	on:keydown|stopPropagation
	class={twMerge(parentClass, 'mb-1 h-8 !rounded-md !shadow-none')}
	{...$$restProps}
/>
