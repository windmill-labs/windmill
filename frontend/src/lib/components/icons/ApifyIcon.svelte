<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
    <!-- Apify logo - simplified version -->
    <circle cx="256" cy="256" r="248" fill="#FF6B35"/>
    <path d="M150 180 L256 120 L362 180 L362 280 L256 340 L150 280 Z" fill="white" stroke="none"/>
    <path d="M200 220 L256 190 L312 220 L312 280 L256 310 L200 280 Z" fill="#FF6B35"/>
    <text x="256" y="275" text-anchor="middle" fill="white" font-family="Arial, sans-serif" font-size="28" font-weight="bold">A</text>
</svg>