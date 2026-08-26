<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #1B1B1B / #FFFFFF per deel.com's own logo_revamp.svg / logo_revamp_white.svg. Post-rebrand the
	period is a square in the wordmark colour, not a blue circle. -->
<svg
	class="text-[#1B1B1B] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-1.311 -2.526 31.474 31.474"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M0 16.0536C0 8.88282 4.57742 5.69628 9.59667 5.69628C14.1776 5.69628 16.1093 8.60687 16.1093 8.60687V0.00960845H21.0708V21.1967C21.0708 22.9359 21.1306 24.5052 21.252 25.9046H16.1109V23.5265C16.1109 23.5265 14.1423 26.411 9.59829 26.411C4.75405 26.411 0 23.6312 0 16.0536ZM10.8639 22.604C14.3778 22.604 16.6497 19.9295 16.6497 16.0536C16.6497 12.0422 14.3762 9.5032 10.8639 9.5032C7.35158 9.5032 5.16209 11.9168 5.16209 16.0536C5.16209 20.1904 7.4438 22.604 10.8639 22.604Z"
		fill="currentColor"
	/>
	<path d="M24.0708 21.2539H28.8501V25.8885H24.0708V21.2539Z" fill="currentColor" />
</svg>
