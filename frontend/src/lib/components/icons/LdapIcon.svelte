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
	<rect x="16" y="16" width="6" height="6" rx="1" />
	<rect x="2" y="16" width="6" height="6" rx="1" />
	<rect x="9" y="2" width="6" height="6" rx="1" />
	<path d="M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3" />
	<path d="M12 12V8" />
</svg>
