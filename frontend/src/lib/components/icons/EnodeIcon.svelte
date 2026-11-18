<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #5D770D / #E8E8E1 per enode.com/static/favicon.svg. Their own favicon carries the pair in a
	prefers-color-scheme block. -->
<svg
	class="text-[#5D770D] dark:text-[#E8E8E1]"
	{width}
	{height}
	viewBox="-0.727 -0.727 17.455 17.455"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		fill="currentColor"
		d="M14.26 6.407c.765 0 1.34.298 1.721.743a8.033 8.033 0 0 0-15.974.054c-.025.25.016.503.12.732a1.761 1.761 0 0 0 3.243-.052.922.922 0 0 0 .06-.286c0-.014 0-.028.004-.041a4.586 4.586 0 0 1 9.071-.363c.382-.467.966-.787 1.755-.787Z"
	/>
	<path
		fill="currentColor"
		d="M14.436 7.204c-.785-.082-1.539.4-1.825 1.109a.484.484 0 0 0-.03.13 4.584 4.584 0 0 1-9.04.496c-.375.54-.994.914-1.853.914C1 9.853.469 9.612.09 9.24a8.03 8.03 0 0 0 15.908-.348.468.468 0 0 0 0-.103c-.095-.802-.745-1.501-1.562-1.585Z"
	/>
	<path fill="currentColor" d="M8.144 9.79a1.864 1.864 0 1 0 0-3.728 1.864 1.864 0 0 0 0 3.729Z" />
</svg>
