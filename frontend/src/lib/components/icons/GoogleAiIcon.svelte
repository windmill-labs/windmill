<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #217BFE → #BD99FE per Google's standard Gemini product icon (gstatic.com/images/branding/productlogos/gemini/v1/192px.svg). Google ships no reversed variant; the same gradient is used on light and dark. -->
<svg
	{width}
	{height}
	viewBox="-0.963 -0.94 25.926 25.926"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<defs>
		<linearGradient
			id="googleai-gemini-gradient"
			x1="7.6"
			y1="15.71"
			x2="18.5"
			y2="6.52"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset="0" stop-color="#217BFE" />
			<stop offset="0.27" stop-color="#078EFB" />
			<stop offset="0.78" stop-color="#A190FF" />
			<stop offset="1" stop-color="#BD99FE" />
		</linearGradient>
	</defs>
	<path
		d="M11.04 19.32Q12 21.51 12 24q0-2.49.93-4.68.96-2.19 2.58-3.81t3.81-2.55Q21.51 12 24 12q-2.49 0-4.68-.93a12.3 12.3 0 0 1-3.81-2.58 12.3 12.3 0 0 1-2.58-3.81Q12 2.49 12 0q0 2.49-.96 4.68-.93 2.19-2.55 3.81a12.3 12.3 0 0 1-3.81 2.58Q2.49 12 0 12q2.49 0 4.68.96 2.19.93 3.81 2.55t2.55 3.81"
		fill="url(#googleai-gemini-gradient)"
	/>
</svg>
