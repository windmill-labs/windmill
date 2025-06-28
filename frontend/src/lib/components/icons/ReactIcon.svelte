<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 600 600" xmlns="http://www.w3.org/2000/svg" version="1.1">
	<g transform="translate(300, 300)">
		<!-- Central circle -->
		<circle fill="#61DAFB" r="50" />

		<!-- Three ellipses -->
		<g stroke="#61DAFB" stroke-width="20" fill="none">
			<ellipse rx="225" ry="90" />
			<ellipse rx="225" ry="90" transform="rotate(60)" />
			<ellipse rx="225" ry="90" transform="rotate(120)" />
		</g>
	</g>
</svg>
