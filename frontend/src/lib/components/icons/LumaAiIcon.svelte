<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per lumalabs.ai (favicon-black.ico on light, favicon-white.ico on dark).
	The two faces ship at 65% opacity, which is what makes their overlap read as a cube. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="0.951 0.795 30.409 30.409"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path d="m4.108 9.033 12.066-6.965v27.865L4.108 22.967V9.033Z" fill-opacity=".65" />
	<path d="M16.174 29.933 4.108 22.967 16.174 16l12.066 6.966-12.066 6.967Z" fill-opacity=".65" />
</svg>
