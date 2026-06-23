<script lang="ts">
	import { goto } from '$app/navigation'
	import { base } from '$app/paths'
	import { page } from '$app/state'
	import { onMount } from 'svelte'

	// WIN-2006: the old same-origin raw-app viewer (`/apps/get_raw/{version}/{path}`,
	// which imported the bundle into the page with no isolation) was removed in favor
	// of the sandboxed unified viewer. Redirect stale bookmarks to the new route,
	// preserving query + hash. The pinned `version` is dropped — the unified viewer
	// always shows the latest, like every other in-workspace app link. Done in the
	// component (not a load redirect) because `url.hash` is unavailable in `load`.
	onMount(() => {
		const path = page.params.path ?? ''
		goto(`${base}/apps_raw/get/${path}${page.url.search}${page.url.hash}`, { replaceState: true })
	})
</script>
