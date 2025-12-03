<script lang="ts">
	interface Props {
		size?: number | string
		color?: string
		strokeWidth?: number | string
		absoluteStrokeWidth?: boolean
		class?: string
		style?: string
	}

	let {
		size = 24,
		color = 'currentColor',
		strokeWidth = 2,
		absoluteStrokeWidth = false,
		class: clazz = '',
		style = ''
	}: Props = $props()

	const computedStrokeWidth = $derived(
		absoluteStrokeWidth ? (Number(strokeWidth) * 24) / Number(size) : strokeWidth
	)
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke={color}
	stroke-width={computedStrokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	class={clazz}
	{style}
>
	<path
		d="M14.517 12.659A2 2 0 0 0 14 14v7a1 1 0 0 1-1.447.895l-2-1A1 1 0 0 1 10 20v-6a2 2 0 0 0-.517-1.341L2.258 4.67A1 1 0 0 1 3 3h8"
	/>
	<path d="m15.523 4.547-.923-.383" />
	<path d="m15.523 6.843-.923.383" />
	<path d="m16.765 2 .382.923" />
	<path d="m17.147 8.467-.383.924" />
	<path d="M19.443 2.923 19.826 2" />
	<path d="m19.825 9.391-.382-.924" />
	<path d="m21.067 4.547.924-.383" />
	<path d="m21.067 6.843.924.383" />
	<circle cx="18.295" cy="5.695" r="3" />
</svg>
