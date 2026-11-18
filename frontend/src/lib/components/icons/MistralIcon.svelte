<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #FA500F per mistral.ai/favicon.svg (mid-band of the #FFAF01 -> #C4001D ramp); drawn here in
	currentColor, the monochrome variant mistral.ai/brand ships. Brand forbids any other recolouring. -->
<svg
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M17.143 3.429v3.428h-3.429v3.429h-3.428V6.857H6.857V3.43H3.43v13.714H0v3.428h10.286v-3.428H6.857v-3.429h3.429v3.429h3.429v-3.429h3.428v3.429h-3.428v3.428H24v-3.428h-3.43V3.429z"
	/>
</svg>
