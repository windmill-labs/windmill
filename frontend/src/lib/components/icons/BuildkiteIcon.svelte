<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #14CC80 / #30F2A2 per buildkite.com/about/brand-assets. Buildkite ships a single mark "for any context", so there is no per-theme variant, and asks that it not be altered. -->
<svg
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path fill="#30F2A2" d="M0 4v8l8 4V8L0 4Z" />
	<path fill="#14CC80" d="M8 8v8l8-4V4L8 8Z" />
	<path fill="#30F2A2" d="M16 4v8l8-4-8-4Z" />
	<path fill="#14CC80" d="M16 12v8l8-4V8l-8 4Z" />
</svg>
