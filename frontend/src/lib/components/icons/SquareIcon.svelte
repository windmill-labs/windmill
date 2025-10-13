<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" {width} {height} viewBox="0 0 30 30" style="fill:currentcolor;">
	<path
		d="M 7 4 C 5.343 4 4 5.343 4 7 L 4 23 C 4 24.657 5.343 26 7 26 L 23 26 C 24.657 26 26 24.657 26 23 L 26 7 C 26 5.343 24.657 4 23 4 L 7 4 z M 8 8 L 22 8 L 22 22 L 8 22 L 8 8 z M 12 12 L 12 18 L 18 18 L 18 12 L 12 12 z"
	/>
</svg>
