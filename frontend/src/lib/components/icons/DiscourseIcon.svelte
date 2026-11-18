<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per discourse.org/brand (discourse-icon.svg / discourse-icon-dark.svg). Only
	the outer bubble reverses; the five inner colours are the same in both variants. -->
<svg
	{width}
	{height}
	class="text-[#000000] dark:text-[#FFFFFF]"
	viewBox="-23.273 -23.273 558.545 558.545"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M258.2 0C119.3 0 2.3 112.6 2.3 251.5V512l255.8-.2c138.9 0 251.5-116.9 251.5-255.8S396.9 0 258.2 0"
	/><path
		d="M260.8 97.4c-54.9 0-105.8 29-133.9 76.2s-29.3 105.7-3.2 154l-28.2 90.8 101.3-22.9c58 26.1 126.1 14.4 172-29.7s60.4-111.6 36.7-170.7c-23.7-59-80.9-97.8-144.6-97.8z"
		style="fill:#fff9ae"
	/><path
		d="M382.7 349.7c-44.3 55.9-120.8 74.6-185.9 45.4L95.5 418.3l103.1-12.2c68.4 40 155.9 22.4 203.4-41s40-152.4-17.6-206.8c43.3 56.7 42.6 135.6-1.7 191.4"
		style="fill:#00aeef"
	/><path
		d="M374 320.1c-38.2 60.2-112.4 86.8-180.1 64.6l-98.3 33.6 101.3-22.9c72.2 32.6 157.3 5.9 197.9-62 40.6-68 23.9-155.6-38.9-203.7 48.8 51.8 56.3 130.3 18.1 190.4"
		style="fill:#00a94f"
	/><path
		d="M133 331c-28.2-68.1-5-146.6 55.8-188.3s142.4-35.2 195.8 15.7C335.2 93.5 244.2 77.7 175.8 122c-68.4 44.4-91.1 133.9-52 205.5l-28.2 90.8z"
		style="fill:#f15d22"
	/><path
		d="M123.7 327.5c-35-64.7-20-145.1 35.9-192.8 56-47.7 137.8-49.9 196.1-5-56-58.9-147.9-65-211.1-13.9S68.1 258 114 325.1l-18.5 93.2z"
		style="fill:#d0232b"
	/>
</svg>
