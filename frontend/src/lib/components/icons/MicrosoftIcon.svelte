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

<!-- #F25022 / #7FBA00 / #00A4EF / #FFB900 per the official logo asset linked from Microsoft's logo third-party usage guidance. Microsoft forbids recolouring the symbol, so it stays full-colour on both themes. -->
<svg
	{width}
	{height}
	class={twMerge(clazz)}
	viewBox="0.727 0.727 510.545 510.545"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path d="M22.3 22.3h222.6v222.6H22.3z" style="fill:#f25022" /><path
		d="M267.1 22.3h222.6v222.6H267.1z"
		style="fill:#7fba00"
	/><path d="M22.3 267.1h222.6v222.6H22.3z" style="fill:#00a4ef" /><path
		d="M267.1 267.1h222.6v222.6H267.1z"
		style="fill:#ffb900"
	/>
</svg>
