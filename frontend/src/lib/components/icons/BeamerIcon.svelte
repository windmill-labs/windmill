<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #1C1E21 / #FFFFFF per the getbeamer.com header logo (g#isotype) and their webclip app icon,
	which sets the same mark in white on #1C1E21. The isotype is monochrome in all first-party artwork. -->
<svg
	class="text-[#1C1E21] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-0.835 -0.835 43.671 43.671"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Beamer"
>
	<g transform="translate(1.66 -3.89)" fill="currentColor">
		<path
			d="M3.3,39.3c-1.1,0-2.1-0.6-2.5-1.7c-0.6-1.4,0.1-3,1.5-3.5l32-13.1c1.4-0.6,3,0.1,3.6,1.5c0.6,1.4-0.1,3-1.5,3.5l-32,13.1 C4,39.2,3.7,39.3,3.3,39.3z"
		/>
		<path
			d="M3.3,28.9c-1.1,0-2.1-0.6-2.5-1.7c-0.6-1.4,0.1-3,1.5-3.5l32-13.1c1.4-0.6,3,0.1,3.6,1.5c0.6,1.4-0.1,3-1.5,3.5l-32,13.1 C4,28.9,3.7,28.9,3.3,28.9z"
		/>
		<path
			d="M3.3,18.2c-1.1,0-2.1-0.6-2.5-1.7c-0.6-1.4,0.1-3,1.5-3.5l19.6-7.9c1.4-0.6,3,0.1,3.6,1.5c0.6,1.4-0.1,3-1.5,3.5L4.4,18 C4,18.2,3.7,18.2,3.3,18.2z"
		/>
		<path
			d="M15.6,44.9c-1.1,0-2.1-0.6-2.5-1.7c-0.6-1.4,0.1-3,1.5-3.5l19.6-7.9c1.4-0.6,3,0.1,3.6,1.5c0.6,1.4-0.1,3-1.5,3.5 l-19.6,7.9C16.3,44.8,16,44.9,15.6,44.9z"
		/>
	</g>
</svg>
