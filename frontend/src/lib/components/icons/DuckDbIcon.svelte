<script lang="ts">
	interface Props {
		width?: number;
		height?: number;
	}

	let { width = 16, height = 16 }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 335 335" fill="none" xmlns="http://www.w3.org/2000/svg">
	<path
		d="M167.2 334.4C74.7 334.4 0 259.7 0 167.2C0 74.7 74.7 0 167.2 0C259.7 0 334.4 74.7 334.4 167.2C334.4 259.7 259.7 334.4 167.2 334.4Z"
		fill="black"
	/>
	<path
		d="M256.6 142.4H223.8V191.9H256.6C270.2 191.9 281.5 180.6 281.5 167C281.5 153.3 270.2 142.4 256.6 142.4Z"
		fill="#FFF100"
	/>
	<path
		d="M63.1 167.2C63.1 205.4 94.2 236.5 132.4 236.5C170.6 236.5 201.7 205.4 201.7 167.2C201.7 129 170.6 97.9 132.4 97.9C94.2 97.9 63.1 129 63.1 167.2Z"
		fill="#FFF100"
	/>
</svg>
