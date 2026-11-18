<script lang="ts">
	interface Props {
		// `size` is what dynamic call sites pass (trigger badges, the native-trigger page,
		// workspace integrations); width/height stay for the ones that pass those instead.
		size?: number
		height?: string | number
		width?: string | number
		class?: string
	}

	let { size = 16, height = undefined, width = undefined, class: clazz = '' }: Props = $props()
</script>

<!-- #4285f4 (with #34a853 / #fbbc05 / #ea4335) per the G mark Google serves in accounts.google.com/gsi/client.
	 developers.google.com/identity/branding-guidelines forbids changing the colour of the G. -->
<svg
	width={width ?? size}
	height={height ?? size}
	class={clazz}
	viewBox="-28.273 -23.273 558.545 558.545"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M501.8 261.8c0-18.2-1.6-35.6-4.7-52.4H256v99.1h137.8c-6.1 31.9-24.2 58.9-51.4 77V450h83.1c48.3-44.6 76.3-110.2 76.3-188.2"
		style="fill:#4285f4"
	/><path
		d="M256 512c69.1 0 127.1-22.8 169.4-61.9l-83.1-64.5c-22.8 15.4-51.9 24.7-86.3 24.7-66.6 0-123.1-44.9-143.4-105.4H27.5V371C69.6 454.5 155.9 512 256 512"
		style="fill:#34a853"
	/><path
		d="M112.6 304.6c-5.1-15.4-8.1-31.7-8.1-48.6s3-33.3 8.1-48.6v-66.1H27.5C10 175.7 0 214.6 0 256s10 80.3 27.5 114.7L93.8 319c0 .1 18.8-14.4 18.8-14.4"
		style="fill:#fbbc05"
	/><path
		d="M256 101.9c37.7 0 71.2 13 98 38.2l73.3-73.3C382.8 25.4 325.1 0 256 0 155.9 0 69.6 57.5 27.5 141.3l85.2 66.1c20.2-60.5 76.7-105.5 143.3-105.5"
		style="fill:#ea4335"
	/><path d="M0 0h512v512H0z" style="fill:none" />
</svg>
