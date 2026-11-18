<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg {width} {height} viewBox="0 3.4 29.2 29.2" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M136.977 0.0457153H0V36H136.977V0.0457153Z" fill="#D40000"/>
<path d="M5.45605 30.5143V27.4057L19.0853 8.89143H5.45605V5.48572H23.7425V8.59429L10.1133 27.1086H23.7425V30.5143H5.45605Z" fill="white"/>
</svg>
