<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFF9F4 per gorgias.com/about-us/style, which ships the symbol as a "Dark"/"Light"
	pair. The guide allows only black or white for the symbol: "Do not use gray!". -->
<svg
	class="text-[#000000] dark:text-[#FFF9F4]"
	{width}
	{height}
	viewBox="-2.085 0.049 69.98 69.98"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M56.8219 17.6032H47.38V8.06662C47.38 5.2675 45.135 3 42.3637 3H9.01633C6.245 3 4 5.2675 4 8.06662V56.3514C4 59.1505 6.245 61.418 9.01633 61.418H52.1976L56.8702 66.1375C57.418 66.6908 58.1538 67 58.9272 67C60.5331 67 61.8382 65.6872 61.8382 64.0598V22.6698C61.8382 19.8707 59.5932 17.6032 56.8219 17.6032ZM57.0206 56.5467H8.81761V7.86591H42.557V17.6032H23.4745C20.7032 17.6032 18.4582 19.8707 18.4582 22.6698V41.7483C18.4582 44.5474 20.7032 46.8149 23.4745 46.8149H42.3637C45.135 46.8149 47.38 44.5474 47.38 41.7483V22.4745H57.0206V56.5521V56.5467ZM42.5624 22.4691V41.9436H23.2812V22.4691H42.5624Z"
		fill="currentColor"
	/>
</svg>
