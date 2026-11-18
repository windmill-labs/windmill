<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- #7EA2EA sampled from adrapid.com's own app favicon; they publish no vector mark or palette.
	Light text darkened from it for legibility. -->
<BrandLetterIcon
	letter="A"
	bgClass="fill-transparent dark:fill-[#7EA2EA]"
	textClass="text-[#5B89E4] dark:text-[#111111]"
	{height}
	{width}
	{size}
/>
