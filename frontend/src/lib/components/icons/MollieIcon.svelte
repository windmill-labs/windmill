<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per Mollie's app icon (my.mollie.com/assets/images/favicons/apple-touch-icon-180x180.png):
	a full-bleed disc with the lowercase m knocked out. The m is the glyph from Mollie-Logo-Black-2023.svg
	(Mollie logo pack, mollie.com/resources), scaled and placed to match that icon pixel for pixel; the logo
	pack itself ships only the 320x94 wordmark. Mollie publishes black and white variants, so the pair flips
	for dark mode. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-8.182 -8.182 196.364 196.364"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Mollie"
>
	<circle cx="90" cy="90" r="90" />
	<path
		class="fill-[#FFFFFF] dark:fill-[#000000]"
		transform="translate(45 29.29) scale(.94142)"
		d="M70.4,30.6c-0.8-0.1-1.6-0.1-2.4-0.1c-7.7,0-15,3.1-20.2,8.7c-5.2-5.5-12.5-8.7-20.1-8.7C12.4,30.5,0,42.9,0,58v34.7h14.9V58.5c0-6.3,5.2-12.1,11.3-12.7c0.4,0,0.9-0.1,1.3-0.1c6.9,0,12.5,5.6,12.5,12.5v34.6h15.2V58.4c0-6.3,5.2-12.1,11.3-12.7c0.4,0,0.9-0.1,1.3-0.1c6.9,0,12.5,5.6,12.6,12.4v34.7h15.2V58.5c0-7-2.6-13.6-7.2-18.8C83.7,34.4,77.3,31.2,70.4,30.6z"
	/>
</svg>
