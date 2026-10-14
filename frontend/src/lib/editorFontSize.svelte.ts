// Reactive Monaco font sizes that follow the global `:root` font-size
// breakpoint set in `frontend/src/lib/assets/app.css` (18px on screens ≥1760px).
// The values mirror Tailwind's `text-xs` (0.75rem) computed pixel size so
// editors visually match the surrounding UI.

const LARGE_SCREEN_QUERY = '(min-device-width: 1760px)'

let isLargeScreen = $state(false)

if (typeof window !== 'undefined') {
	const mq = window.matchMedia(LARGE_SCREEN_QUERY)
	isLargeScreen = mq.matches
	mq.addEventListener('change', (e) => {
		isLargeScreen = e.matches
	})
}

export const editorFontSize = {
	get regular(): number {
		return isLargeScreen ? 13.5 : 12
	},
	get small(): number {
		return isLargeScreen ? 12 : 11
	}
}
