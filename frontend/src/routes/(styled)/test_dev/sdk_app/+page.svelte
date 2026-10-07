<script lang="ts">
	import AppWrapper from '$lib/components/AppWrapper.svelte'
	import type { App } from '$lib/components/apps/types'
	import type { Policy } from '$lib/gen'

	// Auth (workspace + token + user) is wired by the shared TestDevHeader in the
	// test_dev layout. AppEditor self-acquires its UserDraft handle from `path`, so
	// autosave + the indicator engage once a workspace exists (AppWrapper gates the
	// editor on it). Mirrors the React SDK's create mode.

	let app: App = $state({
		grid: [],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		theme: undefined
	})

	let policy: Policy = { execution_mode: 'publisher' }
</script>

<AppWrapper {app} path="u/admin/foo_app" newPath="u/admin/foo_app" summary="" {policy} newApp />
