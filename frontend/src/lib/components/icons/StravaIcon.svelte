<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #FC5200 on both themes per developers.strava.com/guidelines (Strava API logo pack, orange SVGs). Strava's guidelines forbid modifying or altering its logos, and the orange Echelon is the mark Strava itself uses on light and dark alike. -->
<svg
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="#FC5200"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M15.387 17.944l-2.089-4.116h-3.065L15.387 24l5.15-10.172h-3.066m-7.008-5.599l2.836 5.598h4.172L10.463 0l-7 13.828h4.169"
	/>
</svg>
