<script lang="ts">
	import BrandLetterIcon from './BrandLetterIcon.svelte'

	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()
</script>

<!-- Actimo publishes no vector mark and no brand palette; its favicon.ico is zero bytes and the
	pinwheel exists only as raster (actimo.com, now a Kahoot! company). Letter inherits the
	surrounding text colour rather than inventing a brand colour. -->
<BrandLetterIcon
	letter="A"
	bgClass="fill-transparent"
	textClass="text-secondary"
	{height}
	{width}
	{size}
/>
