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
	viewBox="1.057 1.057 21.886 21.886"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
>
	<path d="M12 3v18" />
	<rect width="18" height="18" x="3" y="3" rx="2" />
	<path d="M3 9h18" />
	<path d="M3 15h18" />
</svg>
