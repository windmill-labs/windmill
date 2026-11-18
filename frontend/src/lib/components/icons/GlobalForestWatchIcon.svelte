<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #97BD3D per globalforestwatch.org, which publishes a wordmark rather than a square mark and
	now redirects to Global Nature Watch. -->
<BrandLetterIcon
	letter="G"
	bgClass="fill-transparent dark:fill-[#97BD3D]"
	textClass="text-[#789631] dark:text-[#111111]"
	{height}
	{width}
	{size}
/>
