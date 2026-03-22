<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg">
<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" d="M40.673 30.636a17.694 17.694 0 1 0-33.345 0M4.5 30.635h39M8.791 35.817h15.706m4.393 0h2.982m4.392 0h2.982m-24.64 5.181h19.757"/>
</svg>
