<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #2EA8D7 per aeroworkflow.com's schema.org Organization logo; the mark itself is raster only. -->
<BrandLetterIcon
	letter="A"
	bgClass="fill-transparent dark:fill-[#2EA8D7]"
	textClass="text-[#2596C1] dark:text-[#111111]"
	{height}
	{width}
	{size}
/>
