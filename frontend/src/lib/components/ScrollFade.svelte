<script lang="ts">
	/**
	 * A soft edge on a scroller, so content scrolling out of view fades instead of being
	 * cut against whatever borders it.
	 *
	 * Rendered as an overlay in the scroller's positioned ancestor rather than inside the
	 * scroller: `sticky` would resolve against the scroller's padding box and leave the
	 * first few pixels unfaded. It shows only when there is something hidden in that
	 * direction, so a transcript that fits shows no edge at all.
	 */
	import { twMerge } from 'tailwind-merge'

	interface Props {
		/** The scrolling element this masks. */
		scroller: HTMLElement | undefined
		edge?: 'top' | 'bottom'
		/** Tailwind colour stop to fade from — the surface the scroller sits on. */
		from?: string
		/** Tailwind height of the fade band. */
		height?: string
		class?: string
	}

	let {
		scroller,
		edge = 'top',
		from = 'from-surface',
		height = 'h-4',
		class: className = ''
	}: Props = $props()

	let hidden = $state(true)

	$effect(() => {
		const el = scroller
		if (!el) return
		const update = () => {
			// A pixel of slack: fractional scroll offsets otherwise leave the bottom edge
			// showing on a scroller that is already at its end.
			hidden =
				edge === 'top' ? el.scrollTop <= 1 : el.scrollTop + el.clientHeight >= el.scrollHeight - 1
		}
		update()
		el.addEventListener('scroll', update, { passive: true })
		// Content arriving or the pane resizing changes what is hidden without a scroll.
		const observer = new ResizeObserver(update)
		observer.observe(el)
		if (el.firstElementChild) observer.observe(el.firstElementChild)
		return () => {
			el.removeEventListener('scroll', update)
			observer.disconnect()
		}
	})
</script>

<div
	class={twMerge(
		'pointer-events-none absolute inset-x-0 transition-opacity duration-150',
		edge === 'top' ? 'top-0 bg-gradient-to-b' : 'bottom-0 bg-gradient-to-t',
		from,
		'to-transparent',
		height,
		hidden ? 'opacity-0' : 'opacity-100',
		className
	)}
></div>
