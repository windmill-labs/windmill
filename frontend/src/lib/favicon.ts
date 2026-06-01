import type { Job } from '$lib/gen'

export type JobStatusKind = 'running' | 'success' | 'failure'

const STATUS_COLORS: Record<JobStatusKind, string> = {
	running: '#eab308',
	success: '#22c55e',
	failure: '#ef4444'
}

const DEFAULT_FAVICON = '/logo.svg'

// Inlined windmill logo polygons (fills inlined so the SVG is self-contained as a data URI).
const LOGO_POLYGONS =
	'<polygon fill="#BCD4FC" points="134.78,14.22 114.31,48.21 101.33,69.75 158.22,69.75 177.97,36.95 191.67,14.22"/>' +
	'<polygon fill="#3B82F6" points="227.55,69.75 186.61,69.75 101.33,69.75 129.78,119.02 158.16,119.02 228.61,119.02 256,119.02"/>' +
	'<polygon fill="#3B82F6" points="136.93,132.47 116.46,167.93 73.82,241.78 130.71,241.78 144.9,217.2 180.13,156.18 193.82,132.46"/>' +
	'<polygon fill="#3B82F6" points="121.7,131.95 101.23,96.49 58.59,22.63 30.15,71.91 44.34,96.49 79.57,157.5 93.26,181.22"/>' +
	'<polygon fill="#BCD4FC" points="64.81,131.95 25.15,131.21 0,130.74 28.44,180.01 66.73,180.72 93.26,181.21"/>' +
	'<polygon fill="#BCD4FC" points="165.38,181.74 184.58,216.46 196.75,238.47 225.19,189.2 206.66,155.69 193.83,132.46"/>'

function faviconLink(): HTMLLinkElement {
	let link = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
	if (!link) {
		link = document.createElement('link')
		link.rel = 'icon'
		document.head.appendChild(link)
	}
	return link
}

/** Maps a job to one of the three favicon status colors, following the same
 * discrimination as JobStatusIcon (`'success' in job` => completed). */
export function getJobStatusKind(job: Job | undefined): JobStatusKind | undefined {
	if (!job) return undefined
	if ('success' in job) return job.success ? 'success' : 'failure'
	if (job.canceled) return 'failure'
	return 'running'
}

export function setStatusFavicon(status: JobStatusKind): void {
	const color = STATUS_COLORS[status]
	const svg =
		'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">' +
		`<g>${LOGO_POLYGONS}</g>` +
		'<circle cx="196" cy="196" r="70" fill="#ffffff"/>' +
		`<circle cx="196" cy="196" r="54" fill="${color}"/>` +
		'</svg>'
	faviconLink().href = `data:image/svg+xml,${encodeURIComponent(svg)}`
}

export function resetFavicon(): void {
	faviconLink().href = DEFAULT_FAVICON
}
