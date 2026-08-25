<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #161616 / #FFFFFF per clickhouse.design/brand/logo-usage (logomark, on-light / on-dark). Brand forbids recolouring the mark, so only its own published pair is used. -->
<svg
	xmlns="http://www.w3.org/2000/svg"
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="22.45 19.95 109.2 109.2"
	fill="currentColor"
	class="text-[#161616] dark:text-[#FFFFFF]"
>
	<path
		d="M27,25.7c0-.6.5-1.2,1.2-1.2h8.8c.6,0,1.2.5,1.2,1.2v97.7c0,.6-.5,1.2-1.2,1.2h-8.8c-.6,0-1.2-.5-1.2-1.2V25.7Z"
	/>
	<path
		d="M49.2,25.7c0-.6.5-1.2,1.2-1.2h8.8c.6,0,1.2.5,1.2,1.2v97.7c0,.6-.5,1.2-1.2,1.2h-8.8c-.6,0-1.2-.5-1.2-1.2V25.7Z"
	/>
	<path
		d="M71.4,25.7c0-.6.5-1.2,1.2-1.2h8.8c.6,0,1.2.5,1.2,1.2v97.7c0,.6-.5,1.2-1.2,1.2h-8.8c-.6,0-1.2-.5-1.2-1.2V25.7Z"
	/>
	<path
		d="M93.6,25.7c0-.6.5-1.2,1.2-1.2h8.8c.6,0,1.2.5,1.2,1.2v97.7c0,.6-.5,1.2-1.2,1.2h-8.8c-.6,0-1.2-.5-1.2-1.2V25.7Z"
	/>
	<path
		d="M115.9,64.6c0-.6.5-1.2,1.2-1.2h8.8c.6,0,1.2.5,1.2,1.2v19.9c0,.6-.5,1.2-1.2,1.2h-8.8c-.6,0-1.2-.5-1.2-1.2v-19.9Z"
	/>
</svg>
