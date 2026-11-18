<script lang="ts">
	interface Props {
		height?: string
		width?: string
		/** Accepting `size` is what makes this usable as a Button `startIcon`. */
		size?: number
		class?: string
	}

	let {
		size,
		height = size ? `${size}px` : '24px',
		width = size ? `${size}px` : '24px',
		class: className
	}: Props = $props()
</script>

<!-- #3ECF8E per supabase.com/brand-assets. Forbids modifying or recolouring the mark. -->
<svg
	{width}
	{height}
	class={className}
	viewBox="-6.727 -6.563 181.125 181.125"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M97.4434 164.242C93.2027 169.582 84.6042 166.656 84.502 159.837L83.0078 60.1013H150.07C162.217 60.1013 168.992 74.1309 161.439 83.644L97.4434 164.242Z"
		fill="url(#paint0_linear_210_201)"
	/>
	<path
		d="M97.4434 164.242C93.2027 169.582 84.6042 166.656 84.502 159.837L83.0078 60.1013H150.07C162.217 60.1013 168.992 74.1309 161.439 83.644L97.4434 164.242Z"
		fill="url(#paint1_linear_210_201)"
		fill-opacity="0.2"
	/>
	<path
		d="M70.1702 3.75823C74.411 -1.58278 83.0096 1.3437 83.1118 8.1628L83.7666 107.899H17.5434C5.39617 107.899 -1.37853 93.8692 6.17495 84.3561L70.1702 3.75823Z"
		fill="#3ECF8E"
	/>
	<defs>
		<linearGradient
			id="paint0_linear_210_201"
			x1="83.0078"
			y1="82.2153"
			x2="142.611"
			y2="107.213"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#249361" />
			<stop offset="1" stop-color="#3ECF8E" />
		</linearGradient>
		<linearGradient
			id="paint1_linear_210_201"
			x1="56.5831"
			y1="46.0351"
			x2="83.765"
			y2="97.2037"
			gradientUnits="userSpaceOnUse"
		>
			<stop />
			<stop offset="1" stop-opacity="0" />
		</linearGradient>
	</defs>
</svg>
