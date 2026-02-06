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
	viewBox="0 0 168 168"
	fill={color ?? 'currentColor'}
	xmlns="http://www.w3.org/2000/svg"
	class={clazz}
>
<path d="M157.92 85.75C157.92 80.29 157.43 75.04 156.52 70H84V99.82H125.44C123.62 109.41 118.16 117.53 109.97 122.99V142.38H134.96C149.52 128.94 157.92 109.2 157.92 85.75Z"/>
<path d="M83.9998 161C104.79 161 122.22 154.14 134.96 142.38L109.97 122.99C103.11 127.61 94.3598 130.41 83.9998 130.41C63.9798 130.41 46.9698 116.9 40.8798 98.7H15.2598V118.58C27.9298 143.71 53.8998 161 83.9998 161Z"/>
<path d="M40.88 98.63C39.34 94.01 38.43 89.11 38.43 84C38.43 78.89 39.34 73.99 40.88 69.37V49.49H15.26C10.01 59.85 7 71.54 7 84C7 96.46 10.01 108.15 15.26 118.51L35.21 102.97L40.88 98.63Z"/>
<path d="M83.9998 37.66C95.3398 37.66 105.42 41.58 113.47 49.14L135.52 27.09C122.15 14.63 104.79 7 83.9998 7C53.8998 7 27.9298 24.29 15.2598 49.49L40.8798 69.37C46.9698 51.17 63.9798 37.66 83.9998 37.66Z"/>
</svg>
