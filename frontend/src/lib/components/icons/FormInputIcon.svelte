<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	{width}
	{height}
	viewBox="-0.017 -0.017 24.035 24.035"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
>
	<path d="M12 20h-1a2 2 0 0 1-2-2 2 2 0 0 1-2 2H6" />
	<path d="M13 8h7a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2h-7" />
	<path d="M5 16H4a2 2 0 0 1-2-2v-4a2 2 0 0 1 2-2h1" />
	<path d="M6 4h1a2 2 0 0 1 2 2 2 2 0 0 1 2-2h1" />
	<path d="M9 6v12" />
</svg>
