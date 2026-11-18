<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #0143D9 / #FFFFFF per exa.ai/brand (Exa Brand Assets kit, Logomark Blue/White). Blue for standard applications, white on dark backgrounds. -->
<svg
	class="text-[#0143D9] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="14.915 14.915 120.171 120.171"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<g transform="translate(29.52, 20) scale(0.6044)">
		<path
			d="M150.5 14.1064C150.5 14.3356 150.421 14.5579 150.277 14.736L88.4766 91L150.277 167.264C150.421 167.442 150.5 167.664 150.5 167.894V181C150.5 181.552 150.052 182 149.5 182H1C0.44772 182 0 181.552 0 181V0.999995C0 0.44771 0.447715 0 1 0H149.5C150.052 0 150.5 0.447715 150.5 1V14.1064ZM30.4059 162.719H121.728L76.0664 106.326L30.4059 162.719ZM19.2949 100.261V145.787L56.1572 100.261H19.2949ZM19.2949 80.9801H55.5434L19.2949 36.2121V80.9801ZM76.0664 75.6731L121.728 19.281H30.4059L76.0664 75.6731Z"
		/>
	</g>
</svg>
