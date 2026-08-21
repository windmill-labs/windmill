<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per vercel.com. Vercel ships only light-theme (black) and dark-theme
	(white) triangle marks and explicitly forbids modifying or recoloring the trademarks. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path d="m12 1.608 12 20.784H0Z" />
</svg>
