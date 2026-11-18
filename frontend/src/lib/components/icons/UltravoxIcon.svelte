<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #ff4f68 per the ultravox.ai favicon (framerusercontent.com/images/hzAEdihxJ11mv3l4trNh2WprE.svg).
	 Gradient mark; ultravox.ai links that same asset for both prefers-color-scheme light and dark, so there is no per-theme variant. -->
<svg
	{width}
	{height}
	viewBox="54.091 54.091 231.818 231.818"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<defs>
		<linearGradient
			id="ultravox-gradient-a"
			x1="170.76"
			x2="334.06"
			y1="98.27"
			y2="261.65"
			gradientTransform="matrix(1 0 0 -1 0 342)"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset=".19" stop-color="#bb3b57" />
			<stop offset=".55" stop-color="#ff4f68" />
			<stop offset="1" stop-color="#ffbf61" />
		</linearGradient>
		<linearGradient
			id="ultravox-gradient-b"
			x1="203.91"
			x2="-34.3"
			y1="63.62"
			y2="301.83"
			gradientTransform="matrix(1 0 0 -1 0 342)"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset=".1" stop-color="#ef3254" />
			<stop offset=".25" stop-color="#ff4f68" />
			<stop offset="1" stop-color="#ffbf61" />
		</linearGradient>
	</defs>
	<path
		d="m245.04 238.33 25-12.96a3.992 3.992 0 0 0 2.09-3.51V65.92c0-1.76-2.09-2.59-3.34-1.42l-96.82 96.82 49.16 53.09"
		fill="url(#ultravox-gradient-a)"
	/>
	<path
		d="M245.04 238.33 71.21 64.5c-1.25-1.25-3.34-.33-3.34 1.42v155.94c0 1.51.84 2.84 2.09 3.51l95.57 49.58a9.566 9.566 0 0 0 9.03 0"
		fill="url(#ultravox-gradient-b)"
	/>
	<path d="m172.05 161.32 49.58 53.51-51.67-51.59 2.09-1.92Z" fill="#a22f4b" />
</svg>
