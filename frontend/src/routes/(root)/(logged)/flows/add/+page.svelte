<script lang="ts">
	// `/flows/add` is a thin redirect onto the canonical editor at
	// `/flows/edit/u/{user}/draft_{uuid}?new_draft=true`. See /scripts/add
	// for the design rationale — all editor logic lives in
	// /flows/edit/[...path]. The `u/{user}/` prefix matches Windmill's
	// path scheme so the draft slot lives in the authed user's namespace.
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { userStore } from '$lib/stores'
	import { get } from 'svelte/store'
	import { onMount } from 'svelte'

	onMount(() => {
		const username = get(userStore)?.username ?? 'me'
		const uuid = crypto.randomUUID()
		const params = new URLSearchParams(page.url.searchParams)
		params.set('new_draft', 'true')
		goto(`/flows/edit/u/${username}/draft_${uuid}?${params.toString()}`, {
			replaceState: true
		})
	})
</script>
