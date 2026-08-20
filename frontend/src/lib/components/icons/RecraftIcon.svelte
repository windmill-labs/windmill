<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 plate, #FFFFFF glyph per Recraft's press-kit "Icon White" mark (https://www.recraft.ai/press-releases).
	The plated mark carries its own background: recraft.ai serves it to prefers-color-scheme light and dark alike — not a theme pair. -->
<svg
	{width}
	{height}
	viewBox="-45.455 -45.455 1090.909 1090.909"
	xmlns="http://www.w3.org/2000/svg"
>
	<g clip-path="url(#recraft-clip0_14044_7953)">
		<path d="M1000 0H0V1000H1000V0Z" fill="black" />
		<path
			d="M707.096 588.185H544.157C642.424 557.11 707.935 490.758 707.935 389.972C707.935 265.669 608.829 180 491.244 180C401.376 180 331.666 260.629 331.666 389.972C331.666 431.126 340.065 475.64 350.983 510.916H287.152L225 778H464.368V608.711L563.946 778H812.081L707.096 588.185ZM473.607 496.638C450.93 496.638 429.093 452.963 429.093 389.972C429.093 330.34 450.93 288.346 473.607 288.346C496.284 288.346 518.121 330.34 518.121 389.972C518.121 452.963 496.284 496.638 473.607 496.638Z"
			fill="white"
		/>
	</g>
	<defs>
		<clipPath id="recraft-clip0_14044_7953">
			<rect width="1000" height="1000" fill="white" />
		</clipPath>
	</defs>
</svg>
