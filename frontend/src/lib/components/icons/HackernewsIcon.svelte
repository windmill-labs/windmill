<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 168 168" fill="none" xmlns="http://www.w3.org/2000/svg">
<g clip-path="url(#clip0_210_234)">
<path d="M168 0H0V168H168V0Z" fill="#FB651E"/>
<path d="M78.339 94.9894L49.5029 40.9707H62.6814L79.6438 75.1565C79.9048 75.7654 80.2092 76.396 80.5572 77.0484C80.9051 77.7008 81.2096 78.375 81.4705 79.0709C81.6445 79.3318 81.775 79.571 81.862 79.7885C81.949 80.006 82.0359 80.2017 82.1229 80.3757C82.5579 81.2455 82.9493 82.0936 83.2973 82.92C83.6452 83.7464 83.9496 84.5075 84.2106 85.2034C84.9065 83.7246 85.6676 82.1372 86.494 80.4409C87.3204 78.7447 88.1685 76.9832 89.0384 75.1565L106.262 40.9707H118.527L89.4298 95.6418V130.48H78.339V94.9894Z" fill="white"/>
</g>
<defs>
<clipPath id="clip0_210_234">
<rect width="168" height="168" fill="white"/>
</clipPath>
</defs>
</svg>
