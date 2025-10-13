<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="0 0 168 168"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M91.5447 32.0001L167 75.5642V92.9898L91.5447 136.554V119.128L151.909 84.277L91.545 49.4257L91.5447 32.0001Z"
		fill="url(#paint0_linear_306_215)"
	/>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M91.5447 66.8517L121.727 84.2774L91.5447 101.703V66.8517Z"
		fill="url(#paint1_linear_306_215)"
	/>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M76.4553 32.0016L1 75.5644V92.989L61.363 58.1399V127.841L76.4553 136.555V32.0016ZM46.2738 84.2782L16.0893 101.703L46.2738 119.127V84.2782Z"
		fill="url(#paint2_linear_306_215)"
	/>
	<defs>
		<linearGradient
			id="paint0_linear_306_215"
			x1="1240.56"
			y1="-575.423"
			x2="1259.82"
			y2="-575.423"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#F1007E" />
		</linearGradient>
		<linearGradient
			id="paint1_linear_306_215"
			x1="1240.56"
			y1="-575.423"
			x2="1259.83"
			y2="-575.423"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#F1007E" />
		</linearGradient>
		<linearGradient
			id="paint2_linear_306_215"
			x1="1.00021"
			y1="84.2773"
			x2="76.4547"
			y2="84.2773"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#6D6D6D" />
		</linearGradient>
	</defs>
</svg>
