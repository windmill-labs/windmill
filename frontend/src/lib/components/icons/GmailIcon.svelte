<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #EA4335 per gstatic.com/images/branding/product/2x/gmail_2020q4_48dp.png. Google's brand guidelines forbid recolouring the mark. -->
<svg
	{width}
	{height}
	viewBox="-23.273 -23.273 558.545 558.545"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path d="M34.9 448h81.5V250.2L0 163v250.2C0 432.5 15.7 448 34.9 448" style="fill:#4285f4" /><path
		d="M395.6 448h81.5c19.3 0 34.9-15.7 34.9-34.9V163l-116.4 87.3"
		style="fill:#34a853"
	/><path d="M395.6 99v151.3L512 163v-46.5c0-43.2-49.3-67.8-83.8-41.9" style="fill:#fbbc04" /><path
		d="M116.4 250.2V99L256 203.7 395.6 99v151.3L256 355"
		style="fill:#ea4335"
	/><path d="M0 116.4V163l116.4 87.3V99L83.8 74.5C49.2 48.6 0 73.2 0 116.4" style="fill:#c5221f" />
</svg>
