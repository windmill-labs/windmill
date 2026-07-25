import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'

// The replay page moved to /pipeline_replay (it now replays data-pipeline
// recordings in addition to flow/script ones). Redirect the old path in `load`
// so existing /replay links and bookmarks still resolve instead of 404-ing.
export function load({ url }: { url: URL }) {
	redirect(307, `${base}/pipeline_replay${url.search}`)
}
