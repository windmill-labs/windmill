<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #146EF5 per brand.webflow.com/brand-assets. Mark ships in blue, black or white only. -->
<svg
	{width}
	{height}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="#146EF5"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="m24 4.515-7.658 14.97H9.149l3.205-6.204h-.144C9.566 16.713 5.621 18.973 0 19.485v-6.118s3.596-.213 5.71-2.435H0V4.515h6.417v5.278l.144-.001 2.622-5.277h4.854v5.244h.144l2.72-5.244H24Z"
	/>
</svg>
