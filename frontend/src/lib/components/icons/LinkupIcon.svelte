<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- Linkup's single-stroke logomark, black on light; white on dark backgrounds. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="54.15 28.1 310.8 310.8"
	fill="none"
	stroke="currentColor"
	stroke-width="11.75"
	stroke-linecap="round"
	stroke-linejoin="round"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M73 277.2C76 184.2 105 89.8 163 89.8C205.8 89 236 143.8 248 195C258.8 221.5 260 284.8 269.2 267.5C285 233.5 305 202.8 300.8 158.5C295.2 136 274.5 118 239.8 128C182.5 144 109.2 220 76 272C157.5 245 280.5 210 337 220C360.5 226.5 334.5 250 303.5 260C294.5 265 281 270 262.5 276.5"
	/>
</svg>
