<!-- #FF3737 / #FF7237 / #874FFF / #00B6FF / #24CB71 per static.figma.com/app/icon/2/favicon.svg (2025 brand refresh). Figma's guidelines forbid modifying the marks, so the five-colour original is used on both themes. -->
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
	viewBox="-12.727 -2.727 65.455 65.455"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M0 50C0 44.4772 4.4772 40 10 40H20V50C20 55.5228 15.5228 60 10 60C4.4772 60 0 55.5228 0 50Z"
		fill="#24CB71"
	/>
	<path d="M20 0V20H30C35.5228 20 40 15.5228 40 10C40 4.4772 35.5228 0 30 0H20Z" fill="#FF7237" />
	<circle cx="30" cy="30" r="10" fill="#00B6FF" />
	<path d="M0 10C0 15.5228 4.4772 20 10 20H20V0H10C4.4772 0 0 4.4772 0 10Z" fill="#FF3737" />
	<path d="M0 30C0 35.5228 4.4772 40 10 40H20V20H10C4.4772 20 0 24.4772 0 30Z" fill="#874FFF" />
</svg>
