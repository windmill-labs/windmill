<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg" version="1.1">
	<path
		d="M25.6 2.1875H32L16 29.7875L0 2.1875H6.32H12.24L16 8.5875L19.68 2.1875H25.6Z"
		fill="#41B883"
	/>
	<path d="M0 2.1875L16 29.7875L32 2.1875H25.6L16 18.7475L6.32 2.1875H0Z" fill="#41B883" />
	<path d="M6.32 2.1875L16 18.8275L25.6 2.1875H19.68L16 8.5875L12.24 2.1875H6.32Z" fill="#35495E" />
</svg>
