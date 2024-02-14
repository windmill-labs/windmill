<script lang="ts">
	export let placeholder: string = 'Search...'
	// Using 'any' so 'type="number"' can be passed to the input
	// which should return a number
	export let value: any
	export let debounceDelay: number = 100

	let parentClass: string | undefined = undefined
	export { parentClass as class }

	let timer: NodeJS.Timeout

	function debounce(event: KeyboardEvent): void {
		clearTimeout(timer)

		timer = setTimeout(() => {
			const target = event.target as HTMLInputElement
			value = target.value
		}, debounceDelay)
	}
</script>

<input
	{placeholder}
	on:pointerdown|stopPropagation
	on:keyup={debounce}
	on:keydown|stopPropagation
	class={parentClass}
	{...$$restProps}
/>
