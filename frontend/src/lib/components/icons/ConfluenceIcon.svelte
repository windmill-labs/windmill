<!-- #1868DB tile, white mark, per Atlassian's @atlaskit/logo (atlassian.design logo library). Atlassian requires the logo be used without modification, and its brand appearance is identical in light and dark. -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg {width} {height} viewBox="-1.091 -1.091 26.182 26.182" xmlns="http://www.w3.org/2000/svg">
	<path fill="#1868DB" d="M0 6a6 6 0 0 1 6-6h12a6 6 0 0 1 6 6v12a6 6 0 0 1-6 6H6a6 6 0 0 1-6-6z" />
	<path
		fill="#FFFFFF"
		d="M17.888 14.461c-4.039-1.953-5.219-2.245-6.92-2.245-1.997 0-3.699.831-5.219 3.165l-.25.382c-.204.314-.249.426-.249.561s.068.247.318.404l2.564 1.594c.136.09.25.135.363.135.136 0 .226-.068.363-.27l.408-.628c.635-.966 1.203-1.28 1.928-1.28.636 0 1.385.18 2.315.629l2.677 1.257c.272.134.567.067.704-.247l1.27-2.783c.136-.315.046-.517-.272-.674M6.112 9.545c4.039 1.953 5.219 2.245 6.92 2.245 1.997 0 3.699-.83 5.219-3.165l.25-.381c.204-.315.249-.427.249-.562 0-.134-.068-.247-.318-.404L15.87 5.685c-.137-.09-.25-.135-.364-.135-.136 0-.226.067-.363.27l-.408.628c-.635.965-1.203 1.28-1.928 1.28-.636 0-1.385-.18-2.315-.63L7.814 5.843c-.272-.135-.567-.068-.703.247L5.84 8.872c-.136.314-.045.516.272.673"
	/>
</svg>
