import { on } from 'svelte/events'

/** The scales the zoom buttons and keys step between; the wheel zooms continuously. */
const ZOOM_STEPS = [0.1, 0.25, 0.5, 0.75, 1, 1.5, 2, 3, 4, 6, 8, 12, 16, 24, 32]
const MAX_SCALE = 32
// From this scale up, pixels are drawn as hard squares: smoothing an upscaled
// screenshot or icon blurs exactly the detail the user zoomed in to look at.
const PIXELATED_FROM = 2

type Point = { x: number; y: number }

/**
 * Zoom and pan of one image inside a surface: fit, zoom around a point, step, wheel,
 * drag, double-click and the `+` `-` `0` `1` keys. Attach `surface` to the element that
 * clips the image and `image` to the `<img>`, which `imageStyle` positions at the
 * surface's centre.
 */
export function useZoomPan(opts: {
	/** Room kept around a fitted image, in total per axis. */
	margin: Point
	/** A click that did not drag; `onImage` tells whether it landed on the image. */
	onTap?: (onImage: boolean) => void
}) {
	let natural = $state<{ w: number; h: number } | undefined>(undefined)
	let viewW = $state(0)
	let viewH = $state(0)
	/** `fit` follows the surface as it resizes; a number is a fixed scale of the natural size. */
	let zoom = $state<number | 'fit'>('fit')
	let pan = $state<Point>({ x: 0, y: 0 })
	/** Eases button and keyboard zooms; wheel and drag must track the pointer without lag. */
	let animate = $state(false)
	let panning = $state(false)
	let surfaceEl: HTMLElement | undefined
	let imageEl: HTMLImageElement | undefined

	// Fit never enlarges: a small image is shown at its real size rather than blown up.
	const fitScale = $derived(
		natural && viewW > 0 && viewH > 0
			? Math.min(
					1,
					Math.max(viewW - opts.margin.x, 1) / natural.w,
					Math.max(viewH - opts.margin.y, 1) / natural.h
				)
			: 1
	)
	const scale = $derived(zoom === 'fit' ? fitScale : zoom)
	const minScale = $derived(Math.min(fitScale, ZOOM_STEPS[0]))
	const pannable = $derived(
		!!natural && (natural.w * scale > viewW + 1 || natural.h * scale > viewH + 1)
	)

	function clampPan(p: Point, s: number): Point {
		if (!natural) return { x: 0, y: 0 }
		const maxX = Math.max(0, (natural.w * s - viewW) / 2)
		const maxY = Math.max(0, (natural.h * s - viewH) / 2)
		return {
			x: Math.min(maxX, Math.max(-maxX, p.x)),
			y: Math.min(maxY, Math.max(-maxY, p.y))
		}
	}

	/** `at` is relative to the surface's centre; the image point there stays put. */
	function zoomTo(next: number, at: Point = { x: 0, y: 0 }) {
		const s = Math.min(MAX_SCALE, Math.max(minScale, next))
		const k = s / scale
		pan = clampPan({ x: at.x - (at.x - pan.x) * k, y: at.y - (at.y - pan.y) * k }, s)
		zoom = s
	}

	function fit() {
		animate = true
		zoom = 'fit'
		pan = { x: 0, y: 0 }
	}

	function zoomIn() {
		animate = true
		zoomTo(ZOOM_STEPS.find((z) => z > scale * 1.01) ?? MAX_SCALE)
	}

	function zoomOut() {
		animate = true
		zoomTo(ZOOM_STEPS.findLast((z) => z < scale / 1.01) ?? minScale)
	}

	function actualSize() {
		animate = true
		zoomTo(1)
	}

	function surfacePoint(e: { clientX: number; clientY: number }): Point {
		const r = surfaceEl!.getBoundingClientRect()
		return { x: e.clientX - r.left - r.width / 2, y: e.clientY - r.top - r.height / 2 }
	}

	let drag: { x: number; y: number; pan: Point; moved: boolean; onImage: boolean } | undefined

	function endDrag() {
		drag = undefined
		panning = false
	}

	function surface(node: HTMLElement) {
		surfaceEl = node
		const resize = new ResizeObserver(() => {
			viewW = node.clientWidth
			viewH = node.clientHeight
		})
		resize.observe(node)
		const offs = [
			// Not passive: the page must not scroll under a wheel zoom.
			on(
				node,
				'wheel',
				(e: WheelEvent) => {
					if (!natural) return
					e.preventDefault()
					animate = false
					const lines = e.deltaMode === 1 ? 33 : 1
					// Pinch on a trackpad arrives as ctrl+wheel with small deltas.
					const speed = e.ctrlKey ? 0.01 : 0.0015
					zoomTo(scale * Math.exp(-e.deltaY * lines * speed), surfacePoint(e))
				},
				{ passive: false }
			),
			on(node, 'pointerdown', (e: PointerEvent) => {
				if (e.button !== 0) return
				drag = { x: e.clientX, y: e.clientY, pan, moved: false, onImage: e.target === imageEl }
				node.setPointerCapture(e.pointerId)
			}),
			on(node, 'pointermove', (e: PointerEvent) => {
				if (!drag) return
				const dx = e.clientX - drag.x
				const dy = e.clientY - drag.y
				if (!drag.moved && Math.hypot(dx, dy) < 4) return
				drag.moved = true
				if (pannable) {
					panning = true
					animate = false
					pan = clampPan({ x: drag.pan.x + dx, y: drag.pan.y + dy }, scale)
				}
			}),
			on(node, 'pointerup', () => {
				if (drag && !drag.moved) opts.onTap?.(drag.onImage)
				endDrag()
			}),
			on(node, 'pointercancel', endDrag),
			on(node, 'dblclick', (e: MouseEvent) => {
				if (!natural) return
				animate = true
				if (zoom === 'fit' || Math.abs(scale - fitScale) < 0.001) {
					zoomTo(fitScale < 1 ? 1 : 2, surfacePoint(e))
				} else {
					fit()
				}
			})
		]
		return () => {
			resize.disconnect()
			offs.forEach((off) => off())
			surfaceEl = undefined
		}
	}

	function image(node: HTMLImageElement) {
		imageEl = node
		const measure = () => {
			// An SVG without width/height has no intrinsic size.
			natural = { w: node.naturalWidth || 1024, h: node.naturalHeight || 768 }
		}
		if (node.complete && node.naturalWidth) measure()
		const off = on(node, 'load', measure)
		return () => {
			off()
			imageEl = undefined
		}
	}

	return {
		get natural() {
			return natural
		},
		get scale() {
			return scale
		},
		get minScale() {
			return minScale
		},
		maxScale: MAX_SCALE,
		get zoom() {
			return zoom
		},
		get pannable() {
			return pannable
		},
		get panning() {
			return panning
		},
		/** Positions the image: its natural size, centred, then panned and scaled. */
		get imageStyle() {
			return `width: ${natural?.w ?? 0}px; height: ${natural?.h ?? 0}px; transform: translate(-50%, -50%) translate(${pan.x}px, ${pan.y}px) scale(${scale});`
		},
		get imageClass() {
			return [
				'absolute left-1/2 top-1/2 max-w-none',
				natural ? '' : 'opacity-0',
				animate ? 'transition-transform duration-150 ease-out' : '',
				scale >= PIXELATED_FROM ? '[image-rendering:pixelated]' : ''
			].join(' ')
		},
		surface,
		image,
		fit,
		zoomIn,
		zoomOut,
		actualSize,
		/** Eased, for a zoom the user typed or picked. */
		zoomToAnimated(next: number) {
			animate = true
			zoomTo(next)
		},
		/** Back to fit with no easing, for a viewer shown afresh. */
		reset() {
			zoom = 'fit'
			pan = { x: 0, y: 0 }
			animate = false
		},
		/** Handles a zoom key; returns whether it did, so the caller can prevent its default. */
		handleKey(e: KeyboardEvent): boolean {
			if (e.metaKey || e.ctrlKey || e.altKey) return false
			const action = {
				'+': zoomIn,
				'=': zoomIn,
				'-': zoomOut,
				_: zoomOut,
				'0': fit,
				'1': actualSize
			}[e.key]
			if (!action) return false
			action()
			return true
		}
	}
}
