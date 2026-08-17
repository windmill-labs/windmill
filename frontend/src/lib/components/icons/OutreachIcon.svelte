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
	viewBox="0 0 32 32"
	fill={color ?? 'currentColor'}
	xmlns="http://www.w3.org/2000/svg"
	class={clazz}
>
	<title>outreach-svg</title>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M19.0362 17.8168C17.867 18.907 16.7368 19.3162 15.6412 19.3162C14.5458 19.3162 13.4584 18.8379 12.7668 18.1935C12.0176 17.4949 11.3297 16.334 11.3297 15.1321C11.3297 13.9297 11.8404 13.0832 12.431 12.5327C13.1845 11.8302 14.3149 11.3345 15.9977 11.3345C17.3263 11.3345 18.547 11.6247 19.65 12.653C20.3687 13.3227 20.5204 14.1086 20.5204 14.6446C20.5204 15.3835 20.2054 16.7265 19.0362 17.8168ZM14.7496 6.10764C11.2185 6.10764 8.55804 6.94044 7.01418 8.37972C5.34752 9.93321 4.69444 11.8019 4.69444 13.9376C4.69444 17.3243 6.53258 20.9559 9.09878 23.3486C9.76884 23.9732 12.0832 25.8924 15.1129 25.8924C18.2128 25.8924 20.5172 24.127 21.9657 22.7767C24.7679 20.164 27.3056 15.6592 27.3056 12.7226C27.3056 11.0712 26.4983 9.94429 25.8805 9.36843C23.4451 7.09767 18.2128 6.10764 14.7496 6.10764Z"
	/>
</svg>
