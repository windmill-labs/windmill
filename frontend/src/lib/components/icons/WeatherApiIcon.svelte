<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #72BA2A sampled from weatherapi.com's own touch icon. Their only published logo is a raster
	attribution lockup (droplet plus wordmark), unusable at icon size. -->
<BrandLetterIcon
	letter="W"
	bgClass="fill-transparent dark:fill-[#72BA2A]"
	textClass="text-[#609D23] dark:text-[#111111]"
	{height}
	{width}
	{size}
/>
