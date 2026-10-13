<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- Google's own Chat product icon, gstatic.com/images/branding/productlogos/chat_2026/v2/web/192px.svg.
	Google forbids recolouring its logos, so this ships unmodified apart from namespacing the
	gradient/mask ids, which are document-global. -->
<svg
	{width}
	{height}
	viewBox="0 0 192 192"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Google Chat"
>
	<rect width="160" height="96" x="16" y="28" fill="#00AF57" rx="48" />
	<path
		fill="#0EBC5F"
		d="M133 48c28.167 0 51 22.834 51 51 0 28.167-22.833 51-51 51H96.624l-34.857 23.064c-3.86 2.544-5.789 3.816-7.372 3.92a6 6 0 0 1-5.612-3.022C48 172.583 48 170.271 48 165.649V148.81C25.121 143.78 8 123.39 8 99c0-28.166 22.834-51 51-51h74z"
	/>
	<mask
		id="gchat-mask-a"
		width="176"
		height="129"
		x="8"
		y="48"
		maskUnits="userSpaceOnUse"
		style="mask-type:alpha"
	>
		<path
			fill="#0EBC5F"
			d="M133 48c28.167 0 51 22.834 51 51 0 28.167-22.833 51-51 51H96.722l-39.428 25.896c-3.99 2.62-9.294-.242-9.294-5.015V148.81C25.121 143.78 8 123.39 8 99c0-28.166 22.834-51 51-51h74z"
		/>
	</mask>
	<g mask="url(#gchat-mask-a)">
		<rect width="160" height="96" x="16" y="28" fill="#0EBC5F" rx="48" />
		<rect width="160" height="96" x="16" y="28" fill="url(#gchat-paint-b)" rx="48" />
		<path
			stroke="#fff"
			stroke-linecap="round"
			stroke-width="12"
			d="M62 94s8.84 18 34 18 34-17.182 34-17.182"
		/>
	</g>
	<defs>
		<linearGradient
			id="gchat-paint-b"
			x1="96"
			x2="96"
			y1="28"
			y2="124"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset=".09" stop-color="#94D4FF" />
			<stop offset=".28" stop-color="#78C9FF" />
			<stop offset=".88" stop-color="#01AE58" stop-opacity="0" />
		</linearGradient>
	</defs>
</svg>
