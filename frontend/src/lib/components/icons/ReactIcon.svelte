<script lang="ts">
	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()

	const derivedWidth = $derived(size ? size : width)
	const derivedHeight = $derived(size ? size : height)
</script>

<!-- #087EA4 / #58C4DC per react.dev brand menu (images/brand/logo_light.svg, logo_dark.svg). -->
<svg
	width={derivedWidth}
	height={derivedHeight}
	viewBox="43.04 43.04 513.921 513.921"
	xmlns="http://www.w3.org/2000/svg"
	version="1.1"
	class="text-[#087EA4] dark:text-[#58C4DC]"
>
	<g transform="translate(300, 300)">
		<!-- Central circle -->
		<circle fill="currentColor" r="50" />

		<!-- Three ellipses -->
		<g stroke="currentColor" stroke-width="20" fill="none">
			<ellipse rx="225" ry="90" />
			<ellipse rx="225" ry="90" transform="rotate(60)" />
			<ellipse rx="225" ry="90" transform="rotate(120)" />
		</g>
	</g>
</svg>
