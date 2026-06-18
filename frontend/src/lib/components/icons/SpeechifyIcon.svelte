<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg">
<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" d="M4.5 23.175c6.23.342 8.19-2.093 10.145-4.53q-2.545 6.406.193 9.837q3.381-8.074 10.112-15.113c-3.069 7.033-2.894 14.144-2.415 21.262c2.532-5.437 2.232-11.627 8.824-15.987c-.854 3.991-1.276 7.012-.612 10.583c2.646-6.509 12.753-4.628 12.753-4.628"/>
</svg>
