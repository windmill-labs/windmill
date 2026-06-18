<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg {width} {height} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
	<rect width="24" height="24" rx="4" fill="#017737" />
	<path
		d="M10.5 5C7.74 5 5.5 7.24 5.5 10v8.5h3v-4h1.5c2.76 0 5-2.24 5-5C15 7.24 12.76 5 10 5zm0 8H8.5V10c0-1.1.9-2 2-2s2 .9 2 2-1.4 3-2 3z"
		fill="white"
	/>
</svg>
