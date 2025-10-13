<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="0 0 168 168"
	fill="none"
>
	<path
		d="M0 139.667H18.6667V158.333H0V139.667Z"
		fill="#FF0000"
	/>
	<path
		d="M0 9H18.6667V139.667H0V9ZM37.3333 9H56V158.333H37.3333V9ZM74.6667 9H93.3333V158.333H74.6667V9ZM112 9H130.667V158.333H112V9ZM149.333 69.6667H168V97.6667H149.333V69.6667Z"
		fill="#FFCC00"
	/>
</svg>