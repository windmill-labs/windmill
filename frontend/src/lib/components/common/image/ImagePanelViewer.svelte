<script lang="ts">
	import { Download, Grid2x2, Maximize, Moon, Sun } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import ZoomControl from './ZoomControl.svelte'
	import { twMerge } from 'tailwind-merge'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'

	interface Props {
		src: string
		/** Shown in the header and used as the download's file name. */
		name: string
		/** Byte size, shown beside the dimensions when known. */
		bytes?: number
	}

	let { src, name, bytes }: Props = $props()

	type Backdrop = 'checker' | 'light' | 'dark'
	const BACKDROPS = {
		light: { label: 'Light', icon: Sun },
		dark: { label: 'Dark', icon: Moon },
		checker: { label: 'Checkerboard', icon: Grid2x2 }
	} as const

	const MIN_ZOOM = 0.02
	const MAX_ZOOM = 32
	const ZOOM_STEPS = [
		0.05, 0.1, 0.25, 0.33, 0.5, 0.67, 0.75, 1, 1.25, 1.5, 2, 3, 4, 6, 8, 12, 16, 24, 32
	]
	// Room kept around a fitted image so its edges stay visible against the backdrop.
	const FIT_PADDING = 16
	// From this scale up, pixels are drawn as hard squares: smoothing an upscaled
	// screenshot or icon blurs exactly the detail the user zoomed in to look at.
	const PIXELATED_FROM = 2

	let natural = $state<{ w: number; h: number } | undefined>(undefined)
	// The source that failed, not a flag: a new `src` (the file fixed and re-read) must mount
	// the image again, and only the image's own load event could clear a flag.
	let failedSrc = $state<string>()
	const failed = $derived(failedSrc === src)
	let viewportW = $state(0)
	let viewportH = $state(0)
	// `undefined` is "fit": the scale follows the panel's size. A number is a zoom the
	// user chose, kept as the panel resizes.
	let zoom = $state<number | undefined>(undefined)
	let offset = $state({ x: 0, y: 0 })
	// Until the viewer picks one, the backdrop is Windmill's own theme, and follows it.
	const isDark = useIsDarkMode()
	let chosenBackdrop = $state<Backdrop | undefined>(undefined)
	const backdrop = $derived<Backdrop>(chosenBackdrop ?? (isDark.val ? 'dark' : 'light'))
	// The cycle starts at the theme's own backdrop, so the first click swaps light and dark.
	const backdropCycle = $derived<Backdrop[]>(
		isDark.val ? ['dark', 'light', 'checker'] : ['light', 'dark', 'checker']
	)
	const nextBackdrop = $derived(
		backdropCycle[(backdropCycle.indexOf(backdrop) + 1) % backdropCycle.length]
	)
	let drag: { pointerId: number; x: number; y: number; ox: number; oy: number } | undefined =
		$state(undefined)

	// Fit never enlarges: a small image is shown at its real size rather than blown up.
	const fitScale = $derived(
		natural && viewportW > 0 && viewportH > 0
			? Math.min(
					1,
					Math.max(viewportW - FIT_PADDING * 2, 1) / natural.w,
					Math.max(viewportH - FIT_PADDING * 2, 1) / natural.h
				)
			: 1
	)
	const scale = $derived(zoom ?? fitScale)
	const shownW = $derived(natural ? natural.w * scale : 0)
	const shownH = $derived(natural ? natural.h * scale : 0)
	// A fitted image is centred and stays put; once zoomed it can be dragged anywhere the
	// clamp allows.
	const pannable = $derived(zoom !== undefined)

	function onLoad(e: Event) {
		const img = e.currentTarget as HTMLImageElement
		natural = { w: img.naturalWidth || 1, h: img.naturalHeight || 1 }
	}

	function clampOffset(x: number, y: number, s: number): { x: number; y: number } {
		if (!natural) return { x: 0, y: 0 }
		// The image never leaves the panel: an image larger than it may move until its edge
		// reaches the panel's centre, a smaller one until its centre reaches the panel's edge.
		// Re-centring a side that fits instead would throw away the point the cursor zooms on.
		const maxX = Math.max(natural.w * s, viewportW) / 2
		const maxY = Math.max(natural.h * s, viewportH) / 2
		return {
			x: Math.min(maxX, Math.max(-maxX, x)),
			y: Math.min(maxY, Math.max(-maxY, y))
		}
	}

	/** Zoom to `next`, keeping the image point under (`px`, `py`) — viewport coordinates,
	 * the centre when omitted — where it is. */
	function zoomTo(next: number, px = viewportW / 2, py = viewportH / 2) {
		const s = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next))
		const cx = viewportW / 2 + offset.x
		const cy = viewportH / 2 + offset.y
		const ux = (px - cx) / scale
		const uy = (py - cy) / scale
		offset = clampOffset(px - ux * s - viewportW / 2, py - uy * s - viewportH / 2, s)
		zoom = s
	}

	function stepZoom(direction: 1 | -1) {
		const next =
			direction > 0
				? (ZOOM_STEPS.find((z) => z > scale + 1e-6) ?? MAX_ZOOM)
				: ([...ZOOM_STEPS].reverse().find((z) => z < scale - 1e-6) ?? MIN_ZOOM)
		zoomTo(next)
	}

	function fit() {
		zoom = undefined
		offset = { x: 0, y: 0 }
	}

	function actualSize(px?: number, py?: number) {
		zoomTo(1, px, py)
	}

	function onWheel(e: WheelEvent) {
		if (!natural) return
		e.preventDefault()
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
		// Pixel deltas from a trackpad are small and many, line deltas from a mouse wheel
		// few and large; normalising both keeps one notch near a 10% step.
		const delta = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY
		zoomTo(scale * Math.exp(-delta * 0.0025), e.clientX - rect.left, e.clientY - rect.top)
	}

	function onPointerDown(e: PointerEvent) {
		if (e.button !== 0 || !pannable) return
		;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
		drag = { pointerId: e.pointerId, x: e.clientX, y: e.clientY, ox: offset.x, oy: offset.y }
	}

	function onPointerMove(e: PointerEvent) {
		if (!drag || drag.pointerId !== e.pointerId) return
		offset = clampOffset(drag.ox + e.clientX - drag.x, drag.oy + e.clientY - drag.y, scale)
	}

	function onPointerUp(e: PointerEvent) {
		if (drag?.pointerId === e.pointerId) drag = undefined
	}

	function onDoubleClick(e: MouseEvent) {
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
		if (zoom === undefined && fitScale < 1) actualSize(e.clientX - rect.left, e.clientY - rect.top)
		else fit()
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.metaKey || e.ctrlKey || e.altKey) return
		if (e.key === '+' || e.key === '=') stepZoom(1)
		else if (e.key === '-' || e.key === '_') stepZoom(-1)
		else if (e.key === '0') fit()
		else if (e.key === '1') actualSize()
		else return
		e.preventDefault()
	}

	function humanSize(n: number): string {
		if (n < 1024) return `${n} B`
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
		return `${(n / (1024 * 1024)).toFixed(1)} MB`
	}

	// A 16px checkerboard, the usual stand-in for "transparent".
	const CHECKER_STYLE =
		'background-color:#ffffff;background-image:linear-gradient(45deg,#e5e7eb 25%,transparent 25%,transparent 75%,#e5e7eb 75%),linear-gradient(45deg,#e5e7eb 25%,transparent 25%,transparent 75%,#e5e7eb 75%);background-size:16px 16px;background-position:0 0,8px 8px'
</script>

<div class="flex flex-col h-full min-h-0 bg-surface">
	<!-- The file name is the tab's label already; this bar carries what the tab cannot. -->
	<div
		class="flex flex-wrap items-center justify-between gap-x-2 gap-y-1 px-4 py-2 border-b border-border-light"
	>
		<span class="text-xs text-secondary whitespace-nowrap tabular-nums" title={name}>
			{#if natural}
				{natural.w} × {natural.h}{bytes !== undefined ? ` · ${humanSize(bytes)}` : ''}
			{/if}
		</span>
		<div class="flex flex-wrap items-center gap-1">
			<ZoomControl
				scale={natural ? scale : undefined}
				min={MIN_ZOOM}
				max={MAX_ZOOM}
				onZoomOut={() => stepZoom(-1)}
				onZoomIn={() => stepZoom(1)}
				onZoomTo={(s) => zoomTo(s)}
			/>
			<Button
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{ icon: Maximize }}
				title="Fit to panel (0)"
				disabled={!natural}
				onClick={fit}
			/>
			<Button
				unifiedSize="sm"
				variant={zoom === 1 ? 'default' : 'subtle'}
				title="Actual size (1)"
				disabled={!natural}
				onClick={() => actualSize()}
			>
				1:1
			</Button>
			<Button
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{ icon: BACKDROPS[backdrop].icon }}
				title={`${BACKDROPS[backdrop].label} background, click for ${BACKDROPS[nextBackdrop].label.toLowerCase()}`}
				onClick={() => (chosenBackdrop = nextBackdrop)}
			/>
			<Button
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{ icon: Download }}
				title="Download"
				href={src}
				download={name}
			/>
		</div>
	</div>

	<!-- The viewport takes keys and the wheel itself: zoom follows the cursor, so it has to
	     be the element under it. ARIA's `application` role is the one for a surface that
	     handles its own pointer and keys, but Svelte does not count it as interactive. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
	<div
		class={twMerge(
			'relative flex-1 min-h-0 overflow-hidden outline-none select-none touch-none',
			backdrop === 'light' && 'bg-white',
			// The dark theme's surface-secondary in either theme: the light theme holds it as `-inverse`.
			backdrop === 'dark' && (isDark.val ? 'bg-surface-secondary' : 'bg-surface-secondary-inverse'),
			pannable ? (drag ? 'cursor-grabbing' : 'cursor-grab') : 'cursor-default'
		)}
		style={backdrop === 'checker' ? CHECKER_STYLE : undefined}
		bind:clientWidth={viewportW}
		bind:clientHeight={viewportH}
		tabindex="0"
		role="application"
		aria-roledescription="image viewer"
		aria-label={`${name}. Plus and minus zoom, 0 fits, 1 shows actual size.`}
		onwheel={onWheel}
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		onpointercancel={onPointerUp}
		ondblclick={onDoubleClick}
		onkeydown={onKeyDown}
	>
		{#if failed}
			<div class="absolute inset-0 flex items-center justify-center p-4">
				<span class="rounded-md bg-surface px-3 py-2 text-xs text-secondary shadow-sm">
					This file could not be displayed as an image.
				</span>
			</div>
		{:else}
			<img
				{src}
				alt={name}
				draggable="false"
				onload={onLoad}
				onerror={() => (failedSrc = src)}
				class={twMerge(
					'absolute max-w-none pointer-events-none',
					!natural && 'invisible',
					scale >= PIXELATED_FROM && '[image-rendering:pixelated]'
				)}
				style:width="{shownW}px"
				style:height="{shownH}px"
				style:left="{viewportW / 2 + offset.x - shownW / 2}px"
				style:top="{viewportH / 2 + offset.y - shownH / 2}px"
			/>
		{/if}
	</div>
</div>
