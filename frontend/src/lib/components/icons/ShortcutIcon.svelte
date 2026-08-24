<!-- #494BCB / #797ADE per shortcut.com/branding (mark-default.svg; the reversed lockup uses #797ADE on dark). The mark "is used across various colors but never changes its visual structure." -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	{width}
	{height}
	viewBox="-2.44 -3.651 58.564 58.564"
	fill="none"
	class="text-[#494BCB] dark:text-[#797ADE]"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M17.1304 0L53.64 0L37.1107 17.6479L53.3229 33.3734L36.6202 51.2054L0 51.1965L16.5986 33.4747L0.45036 17.8124L17.1304 0ZM19.5793 36.3657L9.5749 47.0471L30.5973 47.0522L19.5793 36.3657ZM35.6495 46.1685L22.4178 33.335L34.2721 20.6786L47.5036 33.5129L35.6495 46.1685ZM31.2915 17.7875L19.4371 30.444L6.26934 17.6724L18.1229 5.01423L31.2915 17.7875ZM34.1301 14.7568L44.0629 4.15178L23.1968 4.15178L34.1301 14.7568Z"
		fill="currentColor"
	/>
</svg>
