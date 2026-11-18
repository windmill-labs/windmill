<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #131313 / #FFFFFF per design.visma.com/logo (VA symbol from the official Visma Logopack). Positive black on light, negative white on dark; the logo may not be given any other colour. -->
<svg
	{width}
	{height}
	viewBox="-0.964 -33.844 195.686 195.686"
	xmlns="http://www.w3.org/2000/svg"
	class="text-[#131313] dark:text-[#FFFFFF]"
>
	<path
		d="M7.11,7.11h26.68l17.89,87.41h.47L69.89,7.11h26.99l-26.68,113.78h-36.41L7.11,7.11ZM123.56,7.11h36.41l26.68,113.78h-26.68l-17.89-87.41h-.47l-17.73,87.41h-26.99L123.56,7.11Z"
		fill="currentColor"
	/>
</svg>
