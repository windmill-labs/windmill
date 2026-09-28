import { getContext, setContext } from 'svelte'

// A list page mounted somewhere other than its own route — an AI session's preview tab. The
// document URL there belongs to the host, so the page's view (the filters in its query) has to
// live wherever the host keeps it, and its rows and links open where the host decides instead of
// in a drawer or a top-level navigation. Components under such a host read this instead of
// `window.location`; outside one there is none, and they keep to the document.

export type HostedPage = {
	/** The page's query, `''` or `?…`. Reactive: the host may re-point it. */
	readonly search: string
	/** Replace the page's query. */
	setSearch(search: string): void
	/** Open one of the page's rows, by path. */
	openItem(path: string): void
	/** Follow an in-app link (base included) the page would otherwise navigate to. */
	openLink(href: string): void
}

const KEY = Symbol('hostedPage')

/** Reads context: call during component initialisation. */
export function setHostedPage(host: HostedPage): void {
	setContext(KEY, host)
}

/** The host of this page, or undefined on the page's own route. Reads context: call during
 * component initialisation. */
export function useHostedPage(): HostedPage | undefined {
	return getContext<HostedPage | undefined>(KEY)
}
