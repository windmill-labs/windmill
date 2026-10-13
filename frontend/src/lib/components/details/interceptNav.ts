/**
 * Route a link's plain left-click through the detail page's `onNavigate` instead of
 * letting the browser follow it. The detail pages keep real `href`s so a reader can
 * still middle-click or ⌘-click one into a new tab — those clicks are left alone —
 * but a plain click has to reach `onNavigate`, which in an AI session preview moves
 * within the panel rather than taking the browser out of the session.
 */
export function interceptNav(
	onNavigate: (url: string) => void | Promise<void>,
	url: string
): (e?: Event) => void {
	return (e?: Event) => {
		const m = e as MouseEvent | undefined
		if (m && (m.metaKey || m.ctrlKey || m.shiftKey || m.altKey || (m.button ?? 0) !== 0)) return
		e?.preventDefault()
		void onNavigate(url)
	}
}
