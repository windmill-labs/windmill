<script lang="ts">
	interface Props {
		height?: number
		width?: number
	}

	let { height = 24, width = 24 }: Props = $props()
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	{width}
	{height}
	viewBox="0 0 256 256"
	preserveAspectRatio="xMidYMid"
	role="img"
	aria-label="dbt"
>
	<circle cx="128" cy="128" r="120" fill="#FF694A" />
	<!-- The three crossing bars of the dbt mark. -->
	<g stroke="#fff" stroke-width="26" stroke-linecap="round">
		<line x1="128" y1="52" x2="128" y2="204" />
		<line x1="62" y1="90" x2="194" y2="166" />
		<line x1="62" y1="166" x2="194" y2="90" />
	</g>
	<circle cx="128" cy="128" r="30" fill="#FF694A" />
</svg>
