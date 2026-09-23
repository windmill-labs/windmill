// AppPreview must stay a dynamic import: it statically reaches every low-code component
// (and through them monaco, ag-grid, chart.js...), none of which a raw app renders.
// assertLeanPublicAppRoutes in vite.config.js fails the build if that regresses.
let pending: Promise<typeof import('./AppPreview.svelte')> | undefined

export function loadAppPreview() {
	pending ??= import('./AppPreview.svelte').catch((e) => {
		console.error('Could not load the low-code app viewer', e)
		throw e
	})
	return pending
}
