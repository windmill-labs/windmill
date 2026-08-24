<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #1D1B16 / #C7C3B2 per assemblyai.com (assemblyai-logo-full-primary.svg /
	assemblyai-logo-full-secondary.svg). Two colours per theme, so the second stroke carries its
	own fill- utilities: #777673 on light, #FFFFFF on dark. -->
<svg
	class="text-[#1D1B16] dark:text-[#C7C3B2]"
	{width}
	{height}
	viewBox="-1.091 -1.208 26.182 26.182"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M10.595 1.5a3.695 3.695 0 00-3.444 2.355L0 22.26h5.432l5.629-14.486h.002a.96.96 0 011.782 0h.75V4.835h-1.393L13.498 1.5h-2.902z"
	/>
	<path
		class="fill-[#777673] dark:fill-[#FFFFFF]"
		d="M7.151 3.855a3.695 3.695 0 013.26-2.35l-.002-.005H13.405c1.524 0 2.893.936 3.444 2.355L24 22.26h-5.525L11.54 4.413a2.528 2.528 0 00-4.609.006l.22-.564z"
	/>
</svg>
