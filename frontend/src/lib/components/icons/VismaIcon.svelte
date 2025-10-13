<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 36 25" fill="none" xmlns="http://www.w3.org/2000/svg">
	<path
		d="M13.6 0.5C20.6 1.1 28.2 4.6 32.6 9.6C38.2 16 36.3 22.4 28.3 23.9C20.3 25.4 9.30001 21.4 3.70001 15C-0.599994 10.1 -0.499994 5.2 3.40001 2.5L25.6 18.9L13.6 0.5Z"
		fill="#E70641"
	/>
</svg>
