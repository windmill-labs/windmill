<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #004B8D per paychex.com's own logo SVG (themes/custom/paychex2/images/svg/logo-paychex.svg, .st0). The isolated P is the square mark Paychex ships as its own 192x192 app icon. Paychex requires prior approval for any use of its marks. -->
<svg
	{width}
	{height}
	viewBox="-0.965 -0.965 41.932 41.932"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Paychex"
>
	<path
		fill="#004B8D"
		transform="translate(-4.2 -4.55)"
		d="M22.6,15.3h4c2.5,0,5.1,0.5,5.1,3.4c0,2.5-1.6,4.3-6.3,4.3H21L22.6,15.3z M5,43.6h11.6L19,32.1h7.9c10,0,16.5-6.6,16.5-14.3c0-8.4-4.5-12.3-11.5-12.3H13L5,43.6z"
	/>
</svg>
