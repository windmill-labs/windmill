<script lang="ts">
	// `/scripts/add` is a thin redirect onto the canonical editor at
	// `/scripts/edit/draft_{uuid}?new_draft=true`. All editor logic — fetch,
	// seed handling, autosave wiring — lives in `/scripts/edit/[...path]`;
	// keeping it in one place means a single source of truth for the
	// editor's load lifecycle.
	//
	// `new_draft=true` tells the edit page "first mount, don't fetch the
	// non-existent item; seed empty." The flag is consumed and stripped
	// from the URL by the edit page on first render. Any other query
	// params (`template`, `hub`, `tutorial`, ...) ride along untouched so
	// existing entry-points keep working.
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { onMount } from 'svelte'

	onMount(() => {
		const uuid = crypto.randomUUID()
		const params = new URLSearchParams(page.url.searchParams)
		params.set('new_draft', 'true')
		goto(`/scripts/edit/draft_${uuid}?${params.toString()}`, { replaceState: true })
	})
</script>
