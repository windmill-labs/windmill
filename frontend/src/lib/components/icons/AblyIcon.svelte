<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #FF5416 → #FF0000 gradient on both themes per brand.ably.com/logo. "Don't use other colours or gradients for the symbol." -->
<svg
	{width}
	{height}
	viewBox="-3.527 -10.324 84.647 84.647"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	aria-hidden="true"
>
	<path
		d="M38.5717 0L6.29575 59.0738L0 54.6586L29.8639 0H38.5717ZM39.0214 0L71.2973 59.0738L77.5931 54.6586L47.7292 0H39.0214Z"
		fill="url(#ably-motif-upper)"
	/>
	<path
		d="M70.8476 59.4213L38.7965 34.3201L6.74542 59.4213L13.2865 64L38.7965 44.0294L64.3066 64L70.8476 59.4213Z"
		fill="url(#ably-motif-lower)"
	/>
	<defs>
		<linearGradient
			id="ably-motif-upper"
			x1="10.9472"
			y1="74.8439"
			x2="64.9206"
			y2="14.9005"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#FF5416" />
			<stop offset="0.2535" stop-color="#FF5115" />
			<stop offset="0.461" stop-color="#FF4712" />
			<stop offset="0.6523" stop-color="#FF350E" />
			<stop offset="0.8327" stop-color="#FF1E08" />
			<stop offset="1" stop-color="#FF0000" />
		</linearGradient>
		<linearGradient
			id="ably-motif-lower"
			x1="21.4168"
			y1="78.7187"
			x2="53.3166"
			y2="43.2904"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#FF5416" />
			<stop offset="0.2535" stop-color="#FF5115" />
			<stop offset="0.461" stop-color="#FF4712" />
			<stop offset="0.6523" stop-color="#FF350E" />
			<stop offset="0.8327" stop-color="#FF1E08" />
			<stop offset="1" stop-color="#FF0000" />
		</linearGradient>
	</defs>
</svg>
