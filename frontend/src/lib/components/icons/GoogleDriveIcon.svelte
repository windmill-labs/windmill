<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #3186FF / #0EBC5F / #FEC700 gradients per https://www.gstatic.com/images/branding/productlogos/drive_2026/v2/web/192px.svg (Drive 2026 mark, copied verbatim).
	Google's Drive branding guide permits resizing only — no other change to the logo — so no per-theme recolour. -->
<svg
	{width}
	{height}
	viewBox="5.182 4.244 181.636 181.636"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<mask
		id="googledrive-mask"
		width="168"
		height="154"
		x="12"
		y="18"
		maskUnits="userSpaceOnUse"
		style="mask-type:alpha"
	>
		<path
			fill="#b43333"
			d="M63.09 37c14.626-25.333 51.193-25.334 65.819 0l45.033 78c14.626 25.334-3.657 57.001-32.91 57.001H50.967c-29.253 0-47.536-31.667-32.91-57.001z"
		/>
	</mask>
	<g mask="url(#googledrive-mask)">
		<path
			fill="url(#googledrive-yellow)"
			d="M206.905 172.02h-91.888l-19.015-32.934 45.944-79.578z"
		/>
		<path
			fill="url(#googledrive-blue)"
			d="M-14.919 172.006 50.04 59.494v.002L31.032 92.422h38.02L115 172.004l-129.918.001z"
		/>
		<path fill="url(#googledrive-green)" d="M96.007-20.085 141.954 59.5l-19.011 32.928H31.048z" />
	</g>
	<defs>
		<linearGradient
			id="googledrive-yellow"
			x1="193.6"
			x2="103.09"
			y1="165.6"
			y2="111.21"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset=".09" stop-color="#ffe921" />
			<stop offset="1" stop-color="#fec700" />
		</linearGradient>
		<linearGradient
			id="googledrive-blue"
			x1="114.4"
			x2="15.53"
			y1="181.61"
			y2="121.8"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset=".15" stop-color="#a9a8ff" />
			<stop offset=".33" stop-color="#6d97ff" />
			<stop offset=".48" stop-color="#3186ff" />
		</linearGradient>
		<linearGradient
			id="googledrive-green"
			x1="128.88"
			x2="28.7"
			y1="37.88"
			y2="84.64"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset=".55" stop-color="#0ebc5f" />
			<stop offset=".85" stop-color="#78c9ff" />
		</linearGradient>
	</defs>
</svg>
