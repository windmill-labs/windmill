<script lang="ts">
	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height = '24px', width = '24px', size = undefined }: Props = $props()

	const derivedWidth = $derived(size ? size : width)
	const derivedHeight = $derived(size ? size : height)
</script>

<!-- #000000 / #FFFFFF per dcurtis/markdown-mark (public domain). Spec: keep the enclosure's aspect
ratio and radius, keep the M/arrow/box relative sizes, and draw all three in one colour. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	width={derivedWidth}
	height={derivedHeight}
	viewBox="-1.091 -1.091 26.182 26.182"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<g transform="translate(0 4.6154) scale(0.1153846)" fill="currentColor">
		<path
			fill-rule="evenodd"
			clip-rule="evenodd"
			d="m15 10c-2.7614 0-5 2.2386-5 5v98c0 2.761 2.2386 5 5 5h178c2.761 0 5-2.239 5-5v-98c0-2.7614-2.239-5-5-5zm-15 5c0-8.28427 6.71573-15 15-15h178c8.284 0 15 6.71573 15 15v98c0 8.284-6.716 15-15 15h-178c-8.28427 0-15-6.716-15-15z"
		/>
		<path
			d="m30 98v-68h20l20 25 20-25h20v68h-20v-39l-20 25-20-25v39zm125 0-30-33h20v-35h20v35h20z"
		/>
	</g>
</svg>
