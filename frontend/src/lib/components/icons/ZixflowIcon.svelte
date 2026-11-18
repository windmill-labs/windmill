<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #141414 / #FFFFFF per docs.zixflow.com logo pack (logo/light.svg / logo/dark.svg). -->
<svg
	class="text-[#141414] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-1.614 -5.368 38.735 38.735"
	xmlns="http://www.w3.org/2000/svg"
>
	<path d="M2.17188 0H35.5293L18.6889 19.692L19.9551 7.56969L2.17188 0Z" fill="currentColor"
	></path><path d="M33.3598 28H0L16.8403 8.30798L15.5741 20.4303L33.3598 28Z" fill="currentColor"
	></path>
</svg>
