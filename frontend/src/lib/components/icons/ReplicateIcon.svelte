<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per replicate.com header logo (glyph is currentColor; site CSS sets #000, and #FFF under .dark). -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M24 10.262v2.712h-9.518V24h-3.034V10.262zm0-5.131v2.717H8.755V24H5.722V5.131zM24 0v2.717H3.034V24H0V0z"
	/>
</svg>
