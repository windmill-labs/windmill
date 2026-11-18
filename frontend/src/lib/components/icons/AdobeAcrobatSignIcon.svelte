<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #584CCC per adobe.com/cc-shared/assets/img/product-icons/svg/acrobat-sign.svg. Adobe's mark
	itself is not shipped: "Adobe does not allow the use of its product icons by third parties in
	their products or related materials of any kind, except through an Adobe partnership
	agreement" (adobe.com/legal/permissions/icons-web-logos.html). -->
<BrandLetterIcon
	letter="A"
	bgClass="fill-transparent dark:fill-[#584CCC]"
	textClass="text-[#584CCC] dark:text-white"
	{height}
	{width}
	{size}
/>
