<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 140 140">
    <!-- Apify logo icon (extracted from official SVG) -->
    <path fill="#97d700" d="M17,12.59A19.2,19.2,0,0,0,.57,34.25h0l11.36,81.12L59.72,6.62Z"/>
    <path fill="#71c5e8" d="M132.28,80.18l-4.47-62.37A19.22,19.22,0,0,0,107.3.05,9.11,9.11,0,0,0,106,.19L87.57,2.73l40.52,90.85A19.13,19.13,0,0,0,132.28,80.18Z"/>
    <path fill="#ff9013" d="M29.36,135.1A19.06,19.06,0,0,0,39,133.7l69.58-29.36L75.43,30.16Z"/>
</svg>