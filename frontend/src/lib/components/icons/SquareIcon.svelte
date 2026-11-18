<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per Square_Logo_2025 in squareup.com/us/en/press/logo. Square ships only
	black and white logo files and states "Do not change the color", so no tinted variant is allowed. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	xmlns="http://www.w3.org/2000/svg"
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="2.983 2.983 24.035 24.035"
	style="fill:currentcolor;"
>
	<path
		fill-rule="evenodd"
		d="M7.68 4H22.32A3.68 3.68 0 0 1 26 7.68V22.32A3.68 3.68 0 0 1 22.32 26H7.68A3.68 3.68 0 0 1 4 22.32V7.68A3.68 3.68 0 0 1 7.68 4ZM9.16 8H20.84A1.16 1.16 0 0 1 22 9.16V20.84A1.16 1.16 0 0 1 20.84 22H9.16A1.16 1.16 0 0 1 8 20.84V9.16A1.16 1.16 0 0 1 9.16 8ZM12.67 12H17.33A0.67 0.67 0 0 1 18 12.67V17.33A0.67 0.67 0 0 1 17.33 18H12.67A0.67 0.67 0 0 1 12 17.33V12.67A0.67 0.67 0 0 1 12.67 12Z"
	/>
</svg>
