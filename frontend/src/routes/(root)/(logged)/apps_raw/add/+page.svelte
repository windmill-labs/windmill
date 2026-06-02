<script lang="ts">
	// `/apps_raw/add` is a thin redirect onto the canonical editor at
	// `/apps_raw/edit/draft_{uuid}?new_draft=true`. See /scripts/add for the
	// design rationale — all editor logic lives in /apps_raw/edit/[...path].
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { onMount } from 'svelte'

	onMount(() => {
		const uuid = crypto.randomUUID()
		const params = new URLSearchParams(page.url.searchParams)
		params.set('new_draft', 'true')
		goto(`/apps_raw/edit/draft_${uuid}?${params.toString()}`, { replaceState: true })
	})
</script>
