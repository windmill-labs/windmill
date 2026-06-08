<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
	<rect x="1.5" y="1.5" width="21" height="21" rx="4.5" fill="#EB1000" />
	<path
		fill="#fff"
		fill-rule="evenodd"
		d="M10.7 5.8h2.6L17.6 16h-2.7l-.86-2.3h-4.08L9.1 16H6.4L10.7 5.8Zm1.3 2.9-1.4 3.7h2.8L12 8.7Z"
	/>
	<path
		d="M6.6 18.7c1.5.85 2.9.2 4.1-.45s2.5-1.05 4-.2"
		stroke="#fff"
		stroke-width="1.15"
		stroke-linecap="round"
		fill="none"
	/>
</svg>
