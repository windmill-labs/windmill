<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per contiguity.com/assets/icon-white.png and icon-black.png (docs.contiguity.com
	likewise ships logo/black.svg for light and logo/white.svg for dark). The `>_` glyph is knocked out
	to the opposite colour, so it carries its own fill- utilities. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="58.239 57.75 383.523 383.523"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M425 114.923V330.307C425 337.448 422.163 344.296 417.114 349.345C412.065 354.394 405.217 357.23 398.077 357.23H173.438L119.457 404.531C119.392 404.601 119.319 404.664 119.238 404.716C114.404 408.823 108.267 411.077 101.924 411.077C97.9738 411.07 94.0737 410.197 90.498 408.519C85.8472 406.375 81.9122 402.938 79.1638 398.617C76.4154 394.296 74.97 389.274 75.0005 384.153V114.923C75.0005 107.783 77.837 100.935 82.886 95.8856C87.9351 90.8365 94.7831 88 101.924 88H398.077C405.217 88 412.065 90.8365 417.114 95.8856C422.163 100.935 425 107.783 425 114.923Z"
	/>
	<path
		class="fill-[#FFFFFF] dark:fill-[#000000]"
		d="M196.727 231.364L112.182 265.227V236.591L166.045 217.955L165.136 219.318V216.136L166.045 217.5L112.182 198.864V170.227L196.727 204.091V231.364ZM288.801 265V287.273H209.938V265H288.801Z"
	/>
</svg>
