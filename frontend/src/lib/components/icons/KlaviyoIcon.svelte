<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #1D1E20 per klaviyo.com --color-core-charcoal; the flag mark is the standalone logomark
	the site header collapses to, and the shape of klaviyo.com/icons/icon-512x512.png.
	Klaviyo draws it in currentColor, hence the white swap on dark. -->
<svg
	class="text-[#1D1E20] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-0.017 -0.017 24.035 24.035"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Klaviyo"
>
	<path d="M23 19.361H1V4.639h22l-4.617 7.361z" />
</svg>
