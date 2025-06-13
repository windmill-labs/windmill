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
	viewBox="0 0 30 30"
	style="fill:currentcolor;"
>
<path fill="#f35325" d="M0 0h10v10H0z"/><path fill="#81bc06" d="M11 0h10v10H11z"/><path fill="#05a6f0" d="M0 11h10v10H0z"/><path fill="#ffba08" d="M11 11h10v10H11z"/></svg>