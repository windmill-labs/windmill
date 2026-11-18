<script lang="ts">
	interface Props {
		/** Usually the brand's initial. */
		letter: string
		/** Literal Tailwind fill classes for the rounded square behind the letter, e.g.
		 * "fill-transparent dark:fill-[#584CCC]". */
		bgClass: string
		/** Literal Tailwind text classes for the letter, e.g.
		 * "text-[#584CCC] dark:text-white". */
		textClass: string
		height?: string
		width?: string
		size?: number
	}

	let {
		letter,
		bgClass,
		textClass,
		height = '24px',
		width = '24px',
		size = undefined
	}: Props = $props()

	const w = $derived(size ? `${size}px` : width)
	const h = $derived(size ? `${size}px` : height)
</script>

<!-- Stand-in for brands that reserve their logo for licensees: a letter in the brand's own
	colour, rather than their mark or invented artwork.

	On light the letter carries the colour on a bare ground. On dark that inverts to a filled
	rounded square with a light letter, because a mid-tone brand colour chosen to read on white
	goes dim as a foreground on #2e3441 — the filled tile puts the contrast back into the letter.

	Both class props must be written as literal strings at the call site: Tailwind only emits
	classes it can see while scanning, so building them from a variable produces no CSS. -->
<svg
	width={w}
	height={h}
	viewBox="-1.091 -1.091 26.182 26.182"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label={letter}
>
	<rect width="24" height="24" rx="5" class={bgClass} />
	<text
		x="12"
		y="12"
		fill="currentColor"
		class={textClass}
		font-size="17"
		font-weight="700"
		font-family="ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif"
		text-anchor="middle"
		dominant-baseline="central">{letter}</text
	>
</svg>
