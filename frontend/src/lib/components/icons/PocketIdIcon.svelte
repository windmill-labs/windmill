<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		size?: number
		height?: number
		width?: number
		class?: string
	}

	let {
		size = undefined,
		height: heightProp = 24,
		width: widthProp = 24,
		class: clazz = ''
	}: Props = $props()

	const { width, height } = $derived(
		size ? { width: size, height: size } : { width: widthProp, height: heightProp }
	)
</script>

<!-- #000000 / #FFFFFF per pocket-id.org header logo (fill isDark ? #ffffff : #000000) and pocket-id/pocket-id frontend/src/lib/components/logo.svelte. -->
<svg
	xmlns="http://www.w3.org/2000/svg"
	viewBox="-378.618 -738.887 777.775 777.775"
	{width}
	{height}
	fill="currentColor"
	class={twMerge('text-[#000000] dark:text-[#FFFFFF]', clazz)}
>
	<title>pocket-id-svg</title>
	<path
		d="M-250.368,-706.48C-166.912,-706.48 -83.456,-706.48 0,-706.48C149.377,-706.48 270.906,-584.953 270.906,-435.576C270.906,-376.876 252.438,-321.028 217.506,-274.062C183.258,-228.019 136.385,-194.563 81.955,-177.305C76.939,-175.715 71.924,-174.124 66.908,-172.534C54.955,-231.481 43.003,-290.429 31.05,-349.376C34.355,-350.974 37.661,-352.571 40.966,-354.169C73.345,-369.822 94.269,-403.156 94.269,-439.094C94.269,-491.073 51.982,-533.36 0,-533.36C-51.978,-533.36 -94.267,-491.073 -94.267,-439.094C-94.267,-403.156 -73.344,-369.822 -40.963,-354.169C-37.718,-352.6 -34.473,-351.032 -31.228,-349.463C-50.48,-230.815 -69.733,-112.167 -88.985,6.48C-142.779,6.48 -196.574,6.48 -250.368,6.48Z"
	/>
</svg>
