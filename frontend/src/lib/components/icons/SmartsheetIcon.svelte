<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #031C59 (Blue 800) / #FFFFFF per brandguides.brandfolder.com/smartsheet-visual-guide/basics.
	Those are two of the approved logo colorways; the guide forbids any other recolouring. -->
<svg
	class="text-[#031C59] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-7.659 8.591 60.218 60.218"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M8.4,33.6c0,0,2.4-3.3,5.5-3.4c4.9-0.1,8.8,8.6,9.4,9.9c5.4-12,14.3-22.2,21.5-29H0.5c0,4.2,0,46.2,0,49.7 c0,2.7-0.4,4.7-0.6,5.5c2-0.3,9.4-1.6,17-4.3c13.5-4.8,18.7-10,18.8-10l0.3-0.3l0.2,0.4c0,0,1.7,3.7,3.4,5.7 c1.9,2.2,2.9,2.4,3.5,2.3c0.8-0.2,1.3-1.5,1.5-3.5c0.2-2,0.4-6.5,0.4-12c0-9.9,0-23,0-29.6C31.9,31,22.3,55,22.3,55 S16,33.5,8.4,33.6z"
	/>
</svg>
