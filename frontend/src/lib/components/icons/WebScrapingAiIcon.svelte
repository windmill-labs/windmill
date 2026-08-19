<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #209CEE per webscraping.ai; they publish no vector logomark. -->
<BrandLetterIcon
	letter="W"
	bgClass="fill-transparent dark:fill-[#209CEE]"
	textClass="text-[#1190E3] dark:text-[#111111]"
	{height}
	{width}
	{size}
/>
