<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #873EFF / #FFFFFF per the Woo logo pack at woocommerce.com/brand-and-logo-guidelines
	(Woo_logo_color.svg and Woo_logo_white.svg). Automattic requires the mark in its exact,
	most up-to-date form, so only their two published colorways are used, never a recolour. -->
<svg
	{width}
	{height}
	viewBox="-8.345 -76.395 200.291 200.291"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="WooCommerce"
	class="text-[#873EFF] dark:text-[#FFFFFF]"
>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M77.4,0c-4.3,0-7.1,1.4-9.6,6.1L56.4,27.6V8.5c0-5.7-2.7-8.5-7.7-8.5s-7.1,1.7-9.6,6.5L28.3,27.6V8.7 c0-6.1-2.5-8.7-8.6-8.7H7.3C2.6,0,0,2.2,0,6.2s2.5,6.4,7.1,6.4h5.1v24.1c0,6.8,4.6,10.8,11.2,10.8s9.6-2.6,12.9-8.7l7.2-13.5v11.4 c0,6.7,4.4,10.8,11.1,10.8s9.2-2.3,13-8.7l16.6-28C87.8,4.7,85.3,0,77.3,0C77.3,0,77.3,0,77.4,0z"
	/>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M108.6,0C95,0,84.7,10.1,84.7,23.8s10.4,23.7,23.9,23.7s23.8-10.1,23.9-23.7C132.5,10.1,122.1,0,108.6,0z M108.6,32.9c-5.1,0-8.6-3.8-8.6-9.1s3.5-9.2,8.6-9.2s8.6,3.9,8.6,9.2S113.8,32.9,108.6,32.9z"
	/>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M159.7,0c-13.5,0-23.9,10.1-23.9,23.8s10.4,23.7,23.9,23.7s23.9-10.1,23.9-23.7S173.2,0,159.7,0z M159.7,32.9 c-5.2,0-8.5-3.8-8.5-9.1s3.4-9.2,8.5-9.2s8.6,3.9,8.6,9.2S164.9,32.9,159.7,32.9z"
	/>
</svg>
