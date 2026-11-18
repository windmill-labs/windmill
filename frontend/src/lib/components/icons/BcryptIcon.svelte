<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	{width}
	{height}
	viewBox="7.397 7.397 153.205 153.205"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M28 63H140"
		stroke="currentColor"
		stroke-width="14"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M28 105H140"
		stroke="currentColor"
		stroke-width="14"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M70 21L56 147"
		stroke="currentColor"
		stroke-width="14"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M112 21L98 147"
		stroke="currentColor"
		stroke-width="14"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
</svg>
