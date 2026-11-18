<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #2095F3 per the kobotoolbox.org header logo and $kobo-blue in kobotoolbox/kpi jsapp/scss/colors.scss. -->
<svg {width} {height} viewBox="10.416 10.416 54.767 54.767" xmlns="http://www.w3.org/2000/svg">
	<g fill="#2095F3"
		><path
			d="m48 46.6v4.9c0 2.7-2.2 4.9-4.9 4.9h-10.7c-2.7 0-4.9-2.2-4.9-4.9v-27.4c0-2.7 2.2-4.9 4.9-4.9h10.7c2.7 0 4.9 2.2 4.9 4.9v.7c.9-.3 1.9-.5 3-.5 1.2 0 2.4.2 3.5.7v-.8c0-6.3-5.1-11.4-11.4-11.4h-10.7c-6.3 0-11.4 5.1-11.4 11.4v27.3c0 6.3 5.1 11.4 11.4 11.4h10.7c6.3 0 11.4-5.1 11.4-11.4v-12.4z"
		/><path
			d="m42.8 45 11.5-13.2c.4-.4.3-1-.1-1.4-2-1.8-5.1-1.5-6.9.5l-7.1 8.2c-.2.2-.4.2-.6 0l-2.6-3.2c-.5-.6-1.5-.7-2-.1-1.7 1.8-1.8 4.5-.3 6.4l2.2 2.7c1.5 1.9 4.3 2 5.9.1z"
		/></g
	>
</svg>
