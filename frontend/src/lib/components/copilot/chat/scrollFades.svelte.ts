/**
 * Live "is there more past this edge" for one scroll region, so a fade is drawn only over
 * content there actually is: a box short enough not to scroll, or scrolled to an end, shows
 * its first and last line sharp.
 *
 * Measured rather than assumed, because the boxes it serves change height under a still
 * scroll offset — a tool result streams in, a dynamic field fills its options. Put
 * `container` on the scrolling element with `onscroll={measure}`, and `content` on the
 * element inside it whose height moves.
 */
export function scrollFades() {
	let node: HTMLElement | undefined = undefined
	let top = $state(false)
	let bottom = $state(false)
	// Built on first attach, never at call time: this runs during component init, where
	// ResizeObserver does not exist on the server.
	let observer: ResizeObserver | undefined = undefined

	function measure() {
		if (!node) {
			top = false
			bottom = false
			return
		}
		top = node.scrollTop > 1
		bottom = node.scrollHeight - node.scrollTop - node.clientHeight > 1
	}

	function observe(el: HTMLElement) {
		observer ??= new ResizeObserver(measure)
		observer.observe(el)
		return {
			destroy() {
				observer?.unobserve(el)
			}
		}
	}

	return {
		get top() {
			return top
		},
		get bottom() {
			return bottom
		},
		measure,
		container(el: HTMLElement) {
			node = el
			measure()
			const handle = observe(el)
			return {
				destroy() {
					handle.destroy()
					node = undefined
				}
			}
		},
		content: observe
	}
}
