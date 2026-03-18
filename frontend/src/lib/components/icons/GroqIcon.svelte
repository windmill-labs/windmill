<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 33 33" xmlns="http://www.w3.org/2000/svg">
<g clip-path="url(#a)"><path fill="#F43E01" d="M.54.39h32v32h-32z"/><path fill="#fff" d="m18.445 4.406-9.468 13.74 7.341.665-1.69 9.578 9.469-13.74-7.342-.664 1.69-9.579Z"/></g><defs><clipPath id="a"><path fill="#fff" d="M.54.39h32v32h-32z"/></clipPath></defs>
</svg>
