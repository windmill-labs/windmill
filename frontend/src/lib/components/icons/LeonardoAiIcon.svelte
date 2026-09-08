<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #6E60EE per leonardo.ai; they publish no vector logomark. -->
<BrandLetterIcon
	letter="L"
	bgClass="fill-transparent dark:fill-[#6E60EE]"
	textClass="text-[#6E60EE] dark:text-[#FFFFFF]"
	{height}
	{width}
	{size}
/>
