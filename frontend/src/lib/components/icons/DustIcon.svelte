<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #3B82F6 per dust.tt/home/brand-resources (Dust_LogoSquare.svg from their brand kit).
	Dust's guidelines forbid recolouring the logo. -->
<svg {width} {height} viewBox="-2.182 -2.182 52.364 52.364" xmlns="http://www.w3.org/2000/svg">
	<path d="M36 24H24V48H36V24Z" fill="#FE9C1A" />
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M12 36C8.68629 36 6 33.3137 6 30C6 26.6863 8.68629 24 12 24H48V36H12Z"
		fill="#3B82F6"
	/>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M0 48V36H12C15.3137 36 18 38.6863 18 42C18 45.3137 15.3137 48 12 48H0Z"
		fill="#9FDBFF"
	/>
	<path
		d="M12 24C18.6274 24 24 18.6274 24 12C24 5.37258 18.6274 0 12 0C5.37258 0 0 5.37258 0 12C0 18.6274 5.37258 24 12 24Z"
		fill="#E2F78C"
	/>
	<path
		d="M36 24C42.6274 24 48 18.6274 48 12C48 5.37258 42.6274 0 36 0C29.3726 0 24 5.37258 24 12C24 18.6274 29.3726 24 36 24Z"
		fill="#FFC3DF"
	/>
	<path d="M12 0H0V24H12V0Z" fill="#418B5C" />
	<path d="M48 0H24V12H48V0Z" fill="#E14322" />
</svg>
