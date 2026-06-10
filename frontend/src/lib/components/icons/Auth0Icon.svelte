<script lang="ts">
	interface Props {
		size?: number
		height?: number
		width?: number
		color?: string | undefined
		class?: string
	}

	let {
		size = undefined,
		height: heightProp = 24,
		width: widthProp = 24,
		color = undefined,
		class: clazz = ''
	}: Props = $props()

	const { width, height } = $derived(
		size ? { width: size, height: size } : { width: widthProp, height: heightProp }
	)
</script>

<svg
	role="img"
	{width}
	{height}
	viewBox="0 0 256 285"
	fill={color ?? 'currentColor'}
	xmlns="http://www.w3.org/2000/svg"
	class={clazz}
>
	<title>auth0-svg</title>
	<path
		d="M220.412 0H127.997L156.559 89.006H248.975L174.205 142.083L202.775 231.594C250.903 196.534 266.629 143.474 248.983 89.006L220.412 0ZM7.01792 89.006H99.4339L127.997 0H35.5889L7.01792 89.006ZM7.01792 89.006C-10.6371 143.474 5.09792 196.535 53.2249 231.594L81.7879 142.084L7.01792 89.006ZM53.2259 231.594L127.996 284.564L202.766 231.594L127.996 177.747L53.2259 231.594Z"
	/>
</svg>
