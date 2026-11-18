<!-- #0052FF / #FFFFFF per Coinbase's own light/dark logo files (mintcdn.com/coinbase-prod/.../logos/wordmark-light.svg and wordmark-dark.svg, served by docs.cdp.coinbase.com). Coinbase asks that the mark not be altered or recoloured, so only these two published variants are used. -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	class="text-[#0052FF] dark:text-white"
	{width}
	{height}
	viewBox="-1.455 -1.455 34.909 34.909"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		fill="currentColor"
		d="M16.0301 24C11.6018 24 8.01503 20.42 8.01503 16C8.01503 11.58 11.6018 8 16.0301 8C19.9975 8 23.2903 10.8867 23.9249 14.6667H32C31.3187 6.45333 24.4325 0 16.0301 0C7.18013 0 0 7.16667 0 16C0 24.8333 7.18013 32 16.0301 32C24.4325 32 31.3187 25.5467 32 17.3333H23.9249C23.2903 21.1133 19.9975 24 16.0301 24Z"
	/>
</svg>
