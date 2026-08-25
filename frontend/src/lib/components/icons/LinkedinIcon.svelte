<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #0A66C2 per the official inbug SVGs embedded in brand.linkedin.com/in-logo.
	 That page forbids recolouring: only the approved blue, black and white variants. -->
<svg
	{width}
	{height}
	viewBox="-23.273 -23.273 558.545 558.545"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M56.9 512h398.2c31.4 0 56.9-25.5 56.9-56.9V56.9C512 25.5 486.5 0 455.1 0H56.9C25.5 0 0 25.5 0 56.9v398.2C0 486.5 25.5 512 56.9 512"
		style="fill-rule:evenodd;clip-rule:evenodd;fill:#0A66C2"
	/><path
		d="M440.9 440.9h-76V311.5c0-35.5-13.5-55.3-41.6-55.3-30.5 0-46.5 20.6-46.5 55.3v129.4h-73.2V194.4h73.2v33.2s22-40.7 74.3-40.7 89.7 31.9 89.7 98v156zM116.3 162.1c-24.9 0-45.2-20.4-45.2-45.5s20.2-45.5 45.2-45.5 45.1 20.4 45.1 45.5-20.2 45.5-45.1 45.5M78.5 440.9h76.4V194.4H78.5z"
		style="fill-rule:evenodd;clip-rule:evenodd;fill:#fff"
	/>
</svg>
