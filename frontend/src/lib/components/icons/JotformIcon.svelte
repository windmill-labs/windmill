<!-- #0A1551 / #FFFFFF per jotform.com footer logomark (#jotform-logomark-fourth is filled with --jf-logo-img: #0A1551 light, #fff dark). The other three bars keep their fixed brand colours in both themes. -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	{width}
	{height}
	viewBox="-2.743 -1.216 30.485 30.485"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	class="text-[#0A1551] dark:text-[#FFFFFF]"
>
	<path
		fill="#FF6100"
		d="M19.294 3.934a3.341 3.341 0 0 1 4.727 0 3.344 3.344 0 0 1 0 4.728l-11.817 11.82a3.341 3.341 0 0 1-4.726 0 3.344 3.344 0 0 1 0-4.727l11.816-11.82Z"
	/>
	<path
		fill="#0099FF"
		d="M9.25.98a3.344 3.344 0 0 1 4.727 4.728l-8.271 8.274a3.342 3.342 0 0 1-4.727 0 3.344 3.344 0 0 1 0-4.729L9.25.98Z"
	/>
	<path
		fill="#FFB629"
		d="M18.704 17.527a3.344 3.344 0 0 1 4.726 4.728l-4.726 4.729a3.342 3.342 0 1 1-4.727-4.728l4.727-4.729Z"
	/>
	<path
		fill="currentColor"
		d="M1.671 28h4.667c.745 0 1.118-.9.591-1.427L1.426 21.07C.9 20.542 0 20.915 0 21.659v4.67C0 27.251.748 28 1.671 28Z"
	/>
</svg>
