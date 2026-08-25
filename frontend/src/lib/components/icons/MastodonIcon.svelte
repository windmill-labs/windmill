<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- Gradient #6364FF -> #563ACC, both official palette colours per https://joinmastodon.org/branding. Swap to the black or white logo rather than recolouring when contrast fails. -->
<svg
	{width}
	{height}
	viewBox="-23.273 -23.273 558.545 558.545"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<linearGradient
		id="mastodon-a"
		x1="254"
		x2="254"
		y1="513"
		y2="1"
		gradientTransform="matrix(1 0 0 -1 2 513)"
		gradientUnits="userSpaceOnUse"
		><stop offset="0" style="stop-color:#6364ff" /><stop
			offset="1"
			style="stop-color:#563acc"
		/></linearGradient
	><path
		d="M494.3 113.4C486.9 58.3 439 15 382.1 6.6 372.5 5.2 336.2 0 252 0h-.6c-84.2 0-102.2 5.2-111.8 6.6-55.3 8.2-105.7 47.1-118 102.8-5.9 27.4-6.5 57.8-5.4 85.7 1.6 40 1.9 79.9 5.5 119.7 2.5 26.4 6.9 52.7 13.1 78.5 11.6 47.7 58.7 87.4 104.9 103.6 49.4 16.9 102.6 19.7 153.5 8.1 5.6-1.3 11.1-2.8 16.6-4.5 12.4-3.9 26.9-8.3 37.5-16.1.1-.1.3-.2.4-.4s.1-.3.1-.5v-38.6c0-.2 0-.3-.1-.5s-.2-.3-.3-.4-.3-.2-.5-.2h-.5c-32.6 7.8-66.1 11.7-99.6 11.6-57.7 0-73.2-27.4-77.7-38.8-3.6-9.9-5.8-20.1-6.7-30.6 0-.2 0-.4.1-.5.1-.2.2-.3.3-.4s.3-.2.5-.2h.5c32.1 7.7 65 11.6 98 11.6 7.9 0 15.8 0 23.8-.2 33.2-.9 68.2-2.6 100.8-9 .8-.2 1.6-.3 2.3-.5 51.5-9.9 100.5-40.9 105.5-119.5.2-3.1.7-32.4.7-35.6-.1-11 3.4-77.5-.6-118.3"
		style="fill:url(#mastodon-a)"
	/><path
		d="M412.7 175.2v136.7h-54.2V179.2c0-27.9-11.6-42.2-35.3-42.2-26 0-39.1 16.8-39.1 50.1v72.6h-53.8v-72.6c0-33.3-13.1-50.1-39.1-50.1-23.5 0-35.3 14.2-35.3 42.2v132.6h-54.1V175.2c0-27.9 7.1-50.1 21.4-66.6 14.7-16.4 34-24.8 58-24.8 27.7 0 48.7 10.7 62.6 32l13.5 22.6 13.5-22.6c14-21.3 34.9-32 62.6-32 23.9 0 43.2 8.4 58 24.8q21.3 24.6 21.3 66.6"
		style="fill:#fff"
	/>
</svg>
