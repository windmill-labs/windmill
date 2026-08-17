// Reactive Monaco font sizes that follow the global `:root` font-size
// breakpoint set in `frontend/src/lib/assets/app.css` (18px at ≥1760px).
// The values mirror Tailwind's `text-xs` (0.75rem) computed pixel size so
// editors visually match the surrounding UI.

const LARGE_VIEWPORT_QUERY = '(min-width: 1760px)'

let isLargeViewport = $state(false)

if (typeof window !== 'undefined') {
	const mq = window.matchMedia(LARGE_VIEWPORT_QUERY)
	isLargeViewport = mq.matches
	mq.addEventListener('change', (e) => {
		isLargeViewport = e.matches
	})
}

export const editorFontSize = {
	get regular(): number {
		return isLargeViewport ? 13.5 : 12
	},
	get small(): number {
		return isLargeViewport ? 12 : 11
	}
}
