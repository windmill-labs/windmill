<script lang="ts">
	interface Props {
		size?: number
		color?: string | undefined
		class?: string
	}

	let { size = 16, color = undefined, class: clazz = '' }: Props = $props()
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	width={`${size}px`}
	height={`${size}px`}
	viewBox="0 0 24 24"
	fill={color ?? 'currentColor'}
	class={clazz}
>
	<path
		d="M21 3H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h6v2H7v3a1 1 0 0 0 1 1h1v2H7v2h10v-2h-2v-2h1a1 1 0 0 0 1-1v-3h-2v-2h6a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1Zm-1 6H4V5h16v4Zm-9 9v-2h2v2h-2Zm-4-5h10v2H7v-2Z"
	/>
</svg>
