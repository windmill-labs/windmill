<script lang="ts">
	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height, width, size = 24 }: Props = $props()
	let effectiveWidth = $derived(width ?? `${size}px`)
	let effectiveHeight = $derived(height ?? `${size}px`)
</script>

<svg
	width={effectiveWidth}
	height={effectiveHeight}
	viewBox="0 0 24 24"
	xmlns="http://www.w3.org/2000/svg"
	fill="none"
>
	<path
		fill="#5059C9"
		d="M16.148 10.313h5.424c.513 0 .928.414.928.925v4.932a3.408 3.408 0 01-3.411 3.405h-.015a3.408 3.408 0 01-3.411-3.405v-5.372c0-.268.218-.485.485-.485zM19.815 9.338a2.194 2.194 0 002.196-2.194 2.194 2.194 0 00-2.196-2.194 2.194 2.194 0 00-2.197 2.194 2.194 2.194 0 002.197 2.194z"
	/>
	<path
		fill="#7B83EB"
		d="M12.977 9.338a3.17 3.17 0 003.175-3.169A3.17 3.17 0 0012.977 3a3.17 3.17 0 00-3.175 3.169 3.17 3.17 0 003.175 3.169zM17.21 10.313h-8.955a.917.917 0 00-.894.937v5.625A5.503 5.503 0 0012.732 22.5a5.503 5.503 0 005.373-5.625v-5.625a.917.917 0 00-.896-.937z"
	/>
	<path
		fill="#000000"
		d="M13.221 10.313v7.882a.897.897 0 01-.894.893H7.79a5.926 5.926 0 01-.43-2.213v-5.625a.916.916 0 01.895-.937h4.966z"
		opacity=".1"
	/>
	<path
		fill="#000000"
		d="M12.732 10.313v8.37a.9.9 0 01-.894.892H8.02a4.83 4.83 0 01-.4-.975 5.926 5.926 0 01-.258-1.725v-5.625a.916.916 0 01.894-.937h4.477z"
		opacity=".2"
	/>
	<path
		fill="#000000"
		d="M12.732 10.313v7.394a.9.9 0 01-.894.893H7.619a5.926 5.926 0 01-.258-1.725v-5.625a.916.916 0 01.894-.937h4.477z"
		opacity=".2"
	/>
	<path
		fill="#000000"
		d="M12.244 10.313v7.394a.9.9 0 01-.894.893H7.619a5.926 5.926 0 01-.258-1.725v-5.625a.916.916 0 01.894-.937h3.99z"
		opacity=".2"
	/>
	<path
		fill="url(#ms-teams-grad)"
		d="M2.396 7.388h8.953c.496 0 .896.4.896.893v8.938a.894.894 0 01-.896.893H2.396A.894.894 0 011.5 17.22V8.28c0-.494.4-.893.896-.893z"
	/>
	<path fill="#ffffff" d="M9.228 10.79H7.439v4.864h-1.14V10.79H4.515v-.946h4.713v.946z" />
	<defs>
		<linearGradient
			id="ms-teams-grad"
			x1="3.366"
			x2="10.36"
			y1="6.69"
			y2="18.822"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#5A62C3" />
			<stop offset=".5" stop-color="#4D55BD" />
			<stop offset="1" stop-color="#3940AB" />
		</linearGradient>
	</defs>
</svg>
