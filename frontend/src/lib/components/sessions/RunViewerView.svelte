<script lang="ts">
	/*
	 * A run in a session preview tab, rendered by the component the workspace `/run/<id>` page
	 * renders. Everything it reaches is scoped to the run's workspace through `workspace`, and
	 * every navigation it attempts is caught here and turned into a move inside the preview
	 * panel, so nothing takes the browser out of the session.
	 */
	import { setContext, untrack } from 'svelte'
	import RunDetail from '$lib/components/runs/RunDetail.svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import {
		parseHistoricalScriptEdit,
		parsePreviewItemRoute,
		parseRunRoute,
		previewLocationLabel,
		barePath
	} from './previewRouter'

	let {
		runtime,
		jobId,
		workspaceId,
		tabId,
		container,
		reloadNonce = 0
	}: {
		runtime: SessionRuntime
		jobId: string
		workspaceId: string
		/** The preview tab hosting this run, which a move to another run re-points. */
		tabId: string
		/** The tab's host element, which also holds the overlays the page portals out. */
		container: HTMLElement | undefined
		/** Bumped to load the run again from the server. */
		reloadNonce?: number
	} = $props()

	// Mounted in the preview panel, outside the chat's own subtree, as ItemViewerView is: the
	// run page's AI entry points would otherwise offer a session from inside one.
	setContext(
		'aiChatManager',
		untrack(() => runtime.manager)
	)

	// Everything the run page navigates to, resolved against the preview panel. Another run —
	// a re-run, a retry, the parent job — is still this tab's subject, so it re-points in
	// place; an item or any other page opens beside it. Nothing reaches the top-level router.
	function onNavigate(url: string): void {
		const parsed = new URL(url, window.location.origin)
		const href = `${parsed.pathname}${parsed.search}${parsed.hash}`
		if (parseRunRoute(href)) {
			runtime.previewTabs.navigate({ type: 'page', href, label: previewLocationLabel(href) }, tabId)
			return
		}
		const route = parsePreviewItemRoute(href)
		// Legacy drag-and-drop apps and historical script edits have no in-panel host, so they
		// fall through to a page tab. So does `Run again`, whose args ride in the fragment, and
		// a link to one version: opening an item focuses a tab already on it as it stands,
		// where a page tab's URL is written whole, pin and fragment included.
		if (
			route &&
			(route.kind !== 'app' || route.raw_app) &&
			!parseHistoricalScriptEdit(href) &&
			!parsed.hash &&
			!parsed.searchParams.has('version')
		) {
			runtime.previewTabs.open({
				type: 'item',
				item: { path: route.itemPath, summary: '', kind: route.kind, raw_app: route.raw_app },
				mode: route.mode
			})
			return
		}
		runtime.previewTabs.open({ type: 'page', href, label: previewLocationLabel(barePath(href)) })
	}

	// Every in-app link under the tab, not only the ones that call `onNavigate`: a plain anchor
	// followed by the browser would take it out of the session. Listened for on `body`, as
	// ItemViewerView does and for the same reason.
	$effect(() => {
		const el = container
		if (!el) return
		function onClick(e: MouseEvent) {
			if (e.defaultPrevented || e.button !== 0) return
			if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return
			const target = e.target as Element | null
			if (!target || !el!.contains(target)) return
			const a = target.closest?.('a[href]') as HTMLAnchorElement | null
			if (!a || (a.target && a.target !== '_self') || a.hasAttribute('download')) return
			const url = new URL(a.href, window.location.href)
			if (url.origin !== window.location.origin) return
			e.preventDefault()
			if (a.getAttribute('href')?.startsWith('#')) return
			onNavigate(`${url.pathname}${url.search}${url.hash}`)
		}
		document.body.addEventListener('click', onClick)
		return () => document.body.removeEventListener('click', onClick)
	})
</script>

<div class="flex h-full min-h-0 flex-col overflow-auto">
	<!-- Keyed: the run page binds one job's loaders and panels for its lifetime. -->
	{#key `${workspaceId}/${jobId}/${reloadNonce}`}
		<RunDetail id={jobId} workspace={workspaceId} {onNavigate} embedded />
	{/key}
</div>
