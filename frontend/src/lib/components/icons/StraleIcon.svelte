<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #0D0D0E / #F2F2F3 per strale.dev favicon.svg and the site's own --foreground token. Strale's own logo component fills with currentColor, so the mark is meant to take the theme's foreground. -->
<svg
	{width}
	{height}
	viewBox="0.727 0.727 30.545 30.545"
	xmlns="http://www.w3.org/2000/svg"
	class="text-[#0D0D0E] dark:text-[#F2F2F3]"
>
	<path d="M16 2 C 18.2 9, 18.2 14.5, 16 16 C 13.8 14.5, 13.8 9, 16 2 Z" fill="currentColor" />
	<path d="M30 16 C 23 18.2, 17.5 18.2, 16 16 C 17.5 13.8, 23 13.8, 30 16 Z" fill="currentColor" />
	<path d="M16 30 C 13.8 23, 13.8 17.5, 16 16 C 18.2 17.5, 18.2 23, 16 30 Z" fill="currentColor" />
	<path d="M2 16 C 9 13.8, 14.5 13.8, 16 16 C 14.5 18.2, 9 18.2, 2 16 Z" fill="currentColor" />
</svg>
