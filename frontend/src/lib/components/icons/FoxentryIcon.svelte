<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #ff6102 per foxentry.com/assets/img/logo-foxentry-symbol.svg. Fixed tri-tone mark, no per-theme
	variant: the brand ships a separate greyscale logo rather than a recoloured one. -->
<svg {width} {height} viewBox="-10.923 -10.968 262.145 262.145" xmlns="http://www.w3.org/2000/svg">
	<path
		d="M117.23 122.8c7.86 18.81 34.37 55.66 39.58 62.02 3.71 4.59 80.56 54.27 83.49 55.39V0"
		fill="#e74600"
	/><path d="M117.23 122.8l-.16.41-44.19 64.8 43.4 52.2H240.3z" fill="#cc3000" /><path
		d="M44.61 157.34C14.4 187.56 0 240.21 0 240.21h117.23V122.8c0 .09-44.26 6.27-72.62 34.54zM117.23 0v122.8H240.3z"
		fill="#ff6102"
	/>
</svg>
