<script lang="ts">
	interface Props {
		size?: number;
		color?: string | undefined;
		class?: string;
	}

	let { size = 16, color = undefined, class: clazz = '' }: Props = $props();
	
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	width={size}
	height={size}
	viewBox="0 0 64 64"
	class={clazz}
	fill={color ?? 'currentColor'}
	stroke-width="0"
>
	<path
		d="M1.156 50H27.53l15.125 14V50h20.187V0H1.156zm9.937-38.312h11.563l22.25 20.75v-20.75h7.375V38.5H41.094L18.47 17.375V38.5h-7.375z"
	/>
</svg>
