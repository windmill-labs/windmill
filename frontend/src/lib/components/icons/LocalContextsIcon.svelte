<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #F57E20 per localcontexts.org; their mark is published as raster only. -->
<BrandLetterIcon
	letter="L"
	bgClass="fill-transparent dark:fill-[#F57E20]"
	textClass="text-[#E2690A] dark:text-[#111111]"
	{height}
	{width}
	{size}
/>
