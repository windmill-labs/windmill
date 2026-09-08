<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #7E00FF → #07FEEE iridescent sweep, sampled from the logo mark on vectara.com (no brand kit is published). Vectara reserves its trademarks: do not recolour. -->
<svg
	{width}
	{height}
	viewBox="0.144 -0.692 18.113 18.113"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Vectara"
>
	<path
		d="M16.89 6.647c-.267.824-.589 1.63-.962 2.41-.726 1.528-1.594 3.003-2.708 4.268-1.053 1.196-2.442 2.352-4.06 2.593-3.53.524-6.09-3.922-7.198-6.709-.418-1.046-.86-2.122-1.009-3.258C.829 5.008.942 3.308 2.16 3.168c.794-.094 1.388.64 1.86 1.186.562.645.997 1.385 1.485 2.085.69.988 1.844 2.356 3.179 2.188.982-.125 1.747-1.02 2.233-1.834.737-1.23 1.263-2.596 2.048-3.796.584-.9 1.45-1.948 2.541-2.183.804-.175 1.413.238 1.684 1.004.439 1.215.307 2.546 0 3.773q-.13.536-.3 1.056"
		fill="url(#vectara-mark)"
	/>
	<defs>
		<linearGradient
			id="vectara-mark"
			x1="-4.44"
			y1="3.03"
			x2="11.39"
			y2="-7.29"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#7E00FF" />
			<stop offset="0.18" stop-color="#7A00E0" />
			<stop offset="0.34" stop-color="#B4009F" />
			<stop offset="0.46" stop-color="#F0017F" />
			<stop offset="0.58" stop-color="#F25645" />
			<stop offset="0.7" stop-color="#D7CD17" />
			<stop offset="0.84" stop-color="#96FE23" />
			<stop offset="1" stop-color="#07FEEE" />
		</linearGradient>
	</defs>
</svg>
