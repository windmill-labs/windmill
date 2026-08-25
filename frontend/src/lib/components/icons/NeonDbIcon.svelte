<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #37C38F / #34D59A per neon.com/brand (neon-logomark-light-color.svg / neon-logomark-dark-color.svg). Neon forbids recolouring, so only these published variants may be used. -->
<svg
	class="text-[#37C38F] dark:text-[#34D59A]"
	{width}
	{height}
	viewBox="-3.205 -2.892 69.409 69.409"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M63 0.0177909V63.5526L38.4178 42.2501V63.5526H0V0L63 0.0177909ZM7.72251 55.8389H30.6953V25.3238L55.2779 47.0476V7.72922L7.72251 7.71559V55.8389Z"
	/>
</svg>
