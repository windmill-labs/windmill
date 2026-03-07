<script lang="ts">
	interface Props {
		height?: number
		width?: number
	}

	let { height = 24, width = 24 }: Props = $props()
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	{width}
	{height}
	viewBox="0 0 724 561"
>
	<defs>
		<linearGradient id="r-grad" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="#cbced0" />
			<stop offset="1" stop-color="#84838b" />
		</linearGradient>
	</defs>
	<path
		d="M361 0C161.6 0 0 125.5 0 280.5S161.6 561 361 561c111.8 0 211.9-37.8 280.5-97.5l-77-64.5C524 432.7 446.5 453 361 453c-144.4 0-261.5-77.2-261.5-172.5S216.6 108 361 108c96.3 0 181.1 28.7 230.5 71.5l-130 108h262V0H520l-24 41.5C450.8 15.2 408 0 361 0z"
		fill="url(#r-grad)"
	/>
</svg>
