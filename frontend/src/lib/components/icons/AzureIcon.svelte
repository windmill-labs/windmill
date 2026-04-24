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
		d="M22.379 23.343a1.62 1.62 0 0 0 1.536-1.1l.029-.092q.053-.164.081-.336v-.016a1.68 1.68 0 0 0-.268-1.227L15.147 5.44a1.63 1.63 0 0 0-1.354-.724l-3.473.011L5.94 8.57a6 6 0 0 0-1.386 1.74L.262 17.717a1.63 1.63 0 0 0 1.422 2.429l-.055-.013zM13.398 7.25l5.322 9.183-10.683.024zm-3.363 12.754-8.316.021 8.318-14.4 1.795 3.1-6.516 11.274z"
	/>
</svg>
