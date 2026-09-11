/**
 * Offline mode for the public replay page: the recording JSON is the whole data
 * source, so nothing may reach the backend — the visitor has no session and the
 * page may not even be served by the instance the recording came from.
 *
 * Two layers, because the player tree reaches far (LogViewer, DisplayResult,
 * FlowStatusViewer and everything they open):
 *  - components call {@link isReplaying} before fetching, or before rendering
 *    something whose `src`/`href` points at `/api`, so the UI degrades to the
 *    recorded data instead of showing a broken state;
 *  - {@link setOfflineReplay} additionally rejects every generated `*Service`
 *    call at the API client, so a path nobody thought to gate still issues no
 *    request. `EventSource` bypasses that client, but the only ones in the tree
 *    are JobLoader's (short-circuited by `getActiveReplay`) and the recorder's.
 */
import { OpenAPI } from '$lib/gen'
import { getActiveReplay } from './replay.svelte'

let offline = $state(false)

function rejectRequest(): never {
	throw new Error('Offline replay: this page renders a recording and cannot call the API')
}

/** True on the public page only. Use this for recorded *markup* whose rendering
 * would fetch subresources (`<img src>`, map tiles): the threat is content from an
 * arbitrary `?src=` origin on a page that promises to touch nothing, so the answer
 * is to not render it there. In-workspace the recording is one the user opened
 * themselves and the page makes no such promise, so it still renders. */
export function isOfflineReplay(): boolean {
	return offline
}

/** True whenever the UI shows recorded data rather than a live job: the whole
 * public page, or an in-workspace player while it replays a job stream. Use this
 * for anything about *staleness or side effects* — both cases want the recorded
 * value, not a fresh read (re-querying a ducklake table would answer for *now*
 * while the player replays a past run), and neither should let recorded data act. */
export function isReplaying(): boolean {
	return offline || getActiveReplay() != undefined
}

export function setOfflineReplay(on: boolean) {
	if (on === offline) return
	offline = on
	if (on) OpenAPI.interceptors.request.use(rejectRequest)
	else OpenAPI.interceptors.request.eject(rejectRequest)
}
