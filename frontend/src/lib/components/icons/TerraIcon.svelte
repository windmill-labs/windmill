<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #008AFF / #FF3058 / #FFEC80 on #000000 outlines per tryterra.co/providers/terra_icon.svg, the
	only vector square mark Terra ships (the site logo is a "TERRA API" wordmark, the favicon a raster
	.ico). The outlines are part of the artwork and stay black in both themes; the blue T carries the
	mark on dark backgrounds. -->
<svg
	{width}
	{height}
	viewBox="-3.092 1.181 172.638 172.638"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Terra"
>
	<path
		d="M143.795 40.4607V59.9107H130.115H116.465V51.6207V43.3607H108.205H99.9153V82.6707V122.011H106.535H113.155V133.581V145.181H86.2653H59.3453V133.581V122.011H65.9653H72.5853V82.6707V43.3607H65.5553H58.5253V51.6207V59.9107H44.8753H31.1953V40.4607V21.0107H87.4953H143.795V40.4607Z"
		fill="#008AFF"
		stroke="black"
		stroke-width="2"
		stroke-miterlimit="10"
	/>
	<path
		d="M112.631 112.631C150.864 74.3988 170.606 32.154 156.727 18.2748C142.847 4.39555 100.603 24.1376 62.3703 62.37C24.138 100.602 4.39587 142.847 18.2751 156.726C32.1543 170.606 74.3991 150.863 112.631 112.631Z"
		stroke="black"
		stroke-width="2"
		stroke-miterlimit="10"
	/>
	<path
		d="M156.727 156.726C170.606 142.847 150.864 100.602 112.631 62.3699C74.3991 24.1375 32.1543 4.39546 18.2751 18.2747C4.39588 32.1539 24.138 74.3987 62.3703 112.631C100.603 150.863 142.847 170.605 156.727 156.726Z"
		stroke="black"
		stroke-width="2"
		stroke-miterlimit="10"
	/>
	<path
		d="M99.9153 71.1607V43.3607H108.205H116.465V51.6207V59.9107H130.115H143.795V40.4607V21.0107H87.4953H31.1953V40.4607V59.9107H44.8753H58.5253V51.6207V43.3607H65.5553H72.5853V71.1607"
		fill="#008AFF"
		stroke="black"
		stroke-width="2"
		stroke-miterlimit="10"
	/>
	<path
		d="M22.6278 76.3611C32.221 76.3611 39.9978 68.5843 39.9978 58.9911C39.9978 49.3979 32.221 41.6211 22.6278 41.6211C13.0346 41.6211 5.25781 49.3979 5.25781 58.9911C5.25781 68.5843 13.0346 76.3611 22.6278 76.3611Z"
		fill="#FF3058"
		stroke="black"
		stroke-width="2"
		stroke-miterlimit="10"
	/>
	<path
		d="M127.884 107.047L118.642 97.8055C115.319 94.4822 109.931 94.4822 106.608 97.8056L97.3656 107.047C94.0423 110.371 94.0423 115.759 97.3656 119.082L106.608 128.324C109.931 131.648 115.319 131.648 118.642 128.324L127.884 119.082C131.208 115.759 131.208 110.371 127.884 107.047Z"
		fill="#FFEC80"
		stroke="black"
		stroke-width="2"
		stroke-miterlimit="10"
	/>
</svg>
