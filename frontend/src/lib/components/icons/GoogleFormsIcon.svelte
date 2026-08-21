<script lang="ts">
	import { randomUUID } from '$lib/utils/uuid'

	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()

	// SVG ids are document-global: several instances on one page would share the first one's defs.
	const uid = randomUUID()
</script>

<!-- #5746E3 (with #7372FE / #969DFF / #B8C0FF) / no dark variant, per Google's own Forms product icon at www.gstatic.com/images/branding/productlogos/forms_2026/v2/web/192px.svg. Google's brand guidelines forbid modifying or recolouring its product icons. -->
<svg
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="12.955 12.955 166.091 166.091"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<rect width="102" height="50" x="70" y="121" fill="#969dff" rx="25" />
	<rect width="50" height="50" x="20" y="121" fill="#b8c0ff" rx="25" />
	<rect width="50" height="50" x="20" y="71" fill="#5746e3" rx="25" />
	<rect width="102" height="50" x="70" y="71" fill="#5746e3" rx="25" />
	<mask
		id="gforms-mask-{uid}"
		width="102"
		height="50"
		x="70"
		y="21"
		maskUnits="userSpaceOnUse"
		style="mask-type:alpha"
	>
		<rect width="102" height="50" x="70" y="21" fill="#5f54f4" rx="25" />
	</mask>
	<g mask="url(#gforms-mask-{uid})">
		<rect width="102" height="50" x="70" y="21" fill="#7372fe" rx="25" />
		<g filter="url(#gforms-blur-{uid})">
			<circle cx="56" cy="46" r="36" fill="#64afff" />
		</g>
	</g>
	<rect width="50" height="50" x="20" y="21" fill="#7372fe" rx="25" />
	<path stroke="#fff" stroke-linecap="round" stroke-width="12" d="M95 46h52" />
	<circle cx="45" cy="46" r="12" fill="#fff" />
	<defs>
		<filter
			id="gforms-blur-{uid}"
			width="111.47"
			height="111.47"
			x="0.27"
			y="-9.74"
			color-interpolation-filters="sRGB"
			filterUnits="userSpaceOnUse"
		>
			<feFlood flood-opacity="0" result="BackgroundImageFix" />
			<feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
			<feGaussianBlur result="effect1_foregroundBlur" stdDeviation="9.87" />
		</filter>
	</defs>
</svg>
