<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #00758F per mysql.com/common/logos/mysql-logo.svg (the dolphin and "My"). The dolphin itself
	is not shipped: mysql.com/about/legal/trademark.html reserves everything beyond the "Powered by
	MySQL" / "Includes MySQL" conditional-use logos for parties under a signed agreement with
	Oracle. -->
<BrandLetterIcon
	letter="M"
	bgClass="fill-transparent dark:fill-[#00758F]"
	textClass="text-[#00758F] dark:text-white"
	{height}
	{width}
	{size}
/>
