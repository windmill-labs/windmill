<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- Blue500 #0560FF / Neutral0 #FFFFFF per bsky.social/about/support/branding. The downloadable
	media-kit butterfly still ships the older #006AFF, but the palette is the normative source:
	"use only the official color values above. Do not substitute, tint, or approximate." White is
	the approved monochrome variant for dark backgrounds. -->
<svg
	class="text-[#0560FF] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M5.202 2.857C7.954 4.922 10.913 9.11 12 11.358c1.087-2.247 4.046-6.436 6.798-8.501C20.783 1.366 24 .213 24 3.883c0 .732-.42 6.156-.667 7.037-.856 3.061-3.978 3.842-6.755 3.37 4.854.826 6.089 3.562 3.422 6.299-5.065 5.196-7.28-1.304-7.847-2.97-.104-.305-.152-.448-.153-.327 0-.121-.05.022-.153.327-.568 1.666-2.782 8.166-7.847 2.97-2.667-2.737-1.432-5.473 3.422-6.3-2.777.473-5.899-.308-6.755-3.369C.42 10.04 0 4.615 0 3.883c0-3.67 3.217-2.517 5.202-1.026"
	/>
</svg>
