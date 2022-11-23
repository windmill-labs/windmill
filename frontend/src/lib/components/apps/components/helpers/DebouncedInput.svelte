<script lang="ts">
	export let placeholder: string = 'Search...'
	export let value: string
	export let debounceDelay: number = 500

	let timer: NodeJS.Timeout

	function debounce(event: KeyboardEvent): void {
		clearTimeout(timer)

		timer = setTimeout(() => {
			const target = event.target as HTMLInputElement
			value = target.value
		}, debounceDelay)
	}
</script>

<input {placeholder} on:keyup={debounce} />
