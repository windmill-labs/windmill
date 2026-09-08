<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #1D2032 / #EBEDFF per developers.basistheory.com/img/bt-logo-light.svg and bt-logo-dark.svg,
	which ship the same mark geometry in the two theme colours. -->
<svg
	class="text-[#1D2032] dark:text-[#EBEDFF]"
	{width}
	{height}
	viewBox="-1.25 4.95 30.0 30.0"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M26,6.2H1.4c-.8,0-1.4.6-1.4,1.4v5.1c0,.8.6,1.4,1.4,1.4h10.4c1.1,0,2.2.5,3,1.3l3.7,3.8c.7.8,1.1,1.8,1.1,2.8v10.2c0,.8.6,1.4,1.4,1.4h5.1c.8,0,1.4-.6,1.4-1.4V7.6c0-.8-.6-1.4-1.4-1.4Z"
	/>
	<path
		d="M11.6,20c-.3-.3-.6-.4-1-.4H1.4c-.8,0-1.4.6-1.4,1.4v8.7c0,.4.1.7.4,1l2.6,2.6c.3.3.6.4,1,.4h8.7c.8,0,1.4-.6,1.4-1.4v-9.1c0-.4-.1-.7-.4-.9l-2.1-2.2Z"
	/>
</svg>
