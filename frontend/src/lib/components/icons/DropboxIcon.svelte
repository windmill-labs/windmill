<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #0061FE / #0061FE per brand.dropbox.com/logo and the DIG token dig-color__primary__base. Dropbox's branding terms forbid recolouring the logo, and their inverse-theme token keeps the same blue. -->
<svg
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="#0061FE"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M6 1.807L0 5.629l6 3.822 6.001-3.822L6 1.807zM18 1.807l-6 3.822 6 3.822 6-3.822-6-3.822zM0 13.274l6 3.822 6.001-3.822L6 9.452l-6 3.822zM18 9.452l-6 3.822 6 3.822 6-3.822-6-3.822zM6 18.371l6.001 3.822 6-3.822-6-3.822L6 18.371z"
	/>
</svg>
