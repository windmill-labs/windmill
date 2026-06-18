<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
<path d="M2.557 64h22.898c19.082 0 35.988-11.144 35.988-31.828C61.443 8.778 44.193 0 23.547 0h-21v64M16.64 13.014h8.358c11.907 0 21.982 5.877 21.982 18.47 0 14.54-9.846 19.54-22.898 19.54H16.64z" fill="#005cb9"/>
</svg>
