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
	<circle cx="10" cy="7" r="4" />
	<path d="M10.3 15H7a4 4 0 0 0-4 4v2" />
	<path d="M15 15.5V14a2 2 0 0 1 4 0v1.5" />
	<rect width="8" height="5" x="13" y="16" rx=".899" />
</svg>
