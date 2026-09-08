<!-- #8FAF24 on both themes per geldata.com (favicon/apple-touch-icon glyph and its
	<link rel="mask-icon" color>). EdgeDB is now Gel — edgedb.com redirects to geldata.com — so
	this is Gel's "g" symbol, not the retired EDGE|DB wordmark. -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	{width}
	{height}
	viewBox="24.591 19.091 141.818 141.818"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M55 66.0417C55 88.75 73.0914 107.083 95.5 107.083C117.703 107.083 136 88.75 136 66.0417C136 43.5417 117.703 25 95.5 25C73.0914 25 55 43.5417 55 66.0417ZM65.6904 127.5C67.335 141.875 79.0533 155 95.5 155C111.741 155 123.871 141.875 125.515 127.5C127.777 106.667 109.069 113.542 95.5 113.542C81.9315 113.542 63.4289 106.667 65.6904 127.5Z"
		fill="#8FAF24"
	/>
</svg>
