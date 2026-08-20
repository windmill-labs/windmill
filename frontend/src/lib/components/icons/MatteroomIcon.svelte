<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #134A81 / #FFFFFF per the MATTEROOM logomark vector at login.matteroom.com/images/login_logo.svg;
	 square tile proportions taken from their own app icon at matteroom.com/favicon.ico. -->
<svg
	{width}
	{height}
	viewBox="-2.0 -2.0 48.0 48.0"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="MATTEROOM"
>
	<rect width="44" height="44" fill="#134A81" />
	<path
		transform="translate(7 12.212) scale(.769087)"
		fill="#FFFFFF"
		d="M38.1716 11.6645L21.1427 0.3193C20.5737 -0.065 19.8364 -0.0901 19.2315 0.2259C18.6267 0.5527 18.2509 1.1848 18.2509 1.8744V10.5476L2.87914 0.3085C2.31008 -0.0722 1.57996 -0.1045 0.97869 0.2187C0.37384 0.5456 -0.00195 1.1741 -0.00195 1.86V9.2188V12.232V24.1949C-0.00195 24.888 0.55995 25.4519 1.25427 25.4519C1.94502 25.4519 2.50692 24.888 2.50692 24.1949V14.9004L17.8787 25.1394C18.1865 25.3477 18.5444 25.4519 18.9023 25.4519C19.2029 25.4519 19.5036 25.3765 19.7791 25.2292C20.3804 24.906 20.7562 24.2775 20.7562 23.5915V14.8968L36.1137 25.1323C36.425 25.3406 36.7829 25.4411 37.1408 25.4411C37.445 25.4411 37.7457 25.3693 38.0213 25.2184C38.6297 24.8952 39.0055 24.2631 39.0055 23.5736V13.2232C39.0055 12.5983 38.6941 12.0165 38.1716 11.6645"
	/>
</svg>
