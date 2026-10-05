<script lang="ts">
	/*
	 * The View side of a session preview tab: the deployed item, rendered by the same
	 * components the workspace `/scripts/get`, `/flows/get` and `/apps_raw/get` pages
	 * render. Everything those pages reach — drawers, triggers, saved inputs — is scoped
	 * to the session's workspace through `workspace`, and every navigation they attempt
	 * is caught here and turned into a move inside the preview panel, so nothing takes
	 * the browser out of the session.
	 */
	import { untrack } from 'svelte'
	import { base } from '$lib/base'
	import ScriptDetail from '$lib/components/details/ScriptDetail.svelte'
	import FlowDetail from '$lib/components/details/FlowDetail.svelte'
	import InWorkspaceAppViewer from '$lib/components/apps/editor/InWorkspaceAppViewer.svelte'
	import { Button } from '$lib/components/common'
	import { Pen } from 'lucide-svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import type { SessionTargetKind } from './sessionRuntime.svelte'
	import {
		parseHistoricalScriptEdit,
		parsePreviewItemRoute,
		previewLocationLabel,
		stripBase
	} from './previewRouter'

	let {
		runtime,
		kind,
		path,
		version,
		workspaceId,
		tabId,
		container,
		active = true
	}: {
		runtime: SessionRuntime
		/** The preview tab hosting this view, which its in-place moves re-point. */
		tabId: string
		/** The tab's host element, which also holds the overlays the page portals out. */
		container: HTMLElement | undefined
		kind: SessionTargetKind
		path: string
		/** Deployed version this tab is pinned to, from its URL's `?version=`. */
		version?: string
		workspaceId: string
		/** Whether this side is the one on screen. Hidden sides stay mounted, so this gates
		 * the window-level keyboard handler — several instances listen on the window. */
		active?: boolean
	} = $props()

	// This reads the deployed version over the API rather than from the editor cell the
	// chat mutates, so it is the one preview kind that does not self-sync (the invariant
	// `previewReload.ts` states for editors). A deploy from the editor topbar beside it
	// bumps this counter through `itemDeployed`; a deploy from the chat arrives
	// instead as `refresh()`, via `previewReload`'s item list. Neither covers a deploy made
	// outside the session (Compare & Deploy, the CLI, another browser tab).
	const revision = $derived(runtime.deployedRevision(kind, path))
	let reloadKey = $state(0)
	// Seeded from the current value on purpose: before the first run this holds "no deploy
	// seen yet", so a fresh mount must not count as one.
	let lastRevision = untrack(() => revision)
	$effect(() => {
		const rev = revision
		if (rev === lastRevision) return
		lastRevision = rev
		reloadKey++
	})

	/** Refetch the deployed item. The host calls this for the reload signals that reach a
	 * tab from outside it; nothing about becoming visible triggers it, so a form the reader
	 * filled in survives switching tabs away and back. */
	export function refresh(): void {
		reloadKey++
	}

	const searchParams = $derived(version ? new URLSearchParams({ version }) : undefined)

	// Remounts the detail page below, which loads on mount.
	const loadKey = $derived(`${workspaceId}/${kind}/${path}/${version ?? ''}/${reloadKey}`)
	// Nothing deployed at this path. Stamped with the load it answers, so a refetch or a
	// re-pointed tab is never answered from an earlier load's verdict.
	let notFoundFor: string | undefined = $state(undefined)
	const notDeployed = $derived(notFoundFor === loadKey)
	function onLoadState(key: string, state: 'loaded' | 'not_found') {
		// A load that outlived its key (still in flight when the tab was re-pointed) answers
		// for an item no longer on screen, either way.
		if (key !== loadKey) return
		notFoundFor = state === 'not_found' ? key : undefined
	}

	function toEditSide() {
		onNavigate(
			kind === 'flow'
				? `/flows/edit/${path}`
				: kind === 'script'
					? `/scripts/edit/${path}`
					: `/apps_raw/edit/${path}`
		)
	}

	// Everything the detail pages navigate to, resolved against the preview panel: a run
	// opens as its own tab, another side or version of an item re-points this tab, and
	// any other page opens as a page tab. Nothing reaches the top-level router.
	function onNavigate(url: string): void {
		const clean = stripBase(url)
		const query = new URL(url, 'http://x').searchParams
		const run = clean.match(/^\/run\/([^/?#]+)$/)
		if (run) {
			const jobId = decodeURIComponent(run[1])
			runtime.manager.openRunInPreview?.({
				jobId,
				workspace: query.get('workspace') || workspaceId,
				label: previewLocationLabel(`/run/${jobId}`)
			})
			return
		}
		const route = parsePreviewItemRoute(url)
		// Legacy drag-and-drop apps and historical script edits have no in-panel host, so they
		// fall through to a page tab.
		if (route && (route.kind !== 'app' || route.raw_app) && !parseHistoricalScriptEdit(url)) {
			// A script's version history addresses a version by putting its hash where the
			// path goes. Every real script path is `u/<user>/…` or `f/<folder>/…`, so a
			// segment with no slash is a hash — pinned in the query instead, or this tab's
			// identity would become that hash and it would stop being a tab on this script.
			const isVersionHash = route.kind === 'script' && !route.itemPath.includes('/')
			const target = {
				type: 'item' as const,
				item: {
					path: isVersionHash ? path : route.itemPath,
					summary: '',
					kind: route.kind,
					raw_app: route.raw_app
				},
				mode: route.mode,
				version: isVersionHash ? route.itemPath : (query.get('version') ?? undefined)
			}
			// This tab is the deployed side, so a link to an editor is a request for the other
			// side: it opens beside this one rather than replacing the page it was asked from.
			// Everything else the page links — another version, the head, a rename's new path —
			// is still this same view of the item, so it re-points in place.
			if (route.mode === 'edit') runtime.previewTabs.open(target)
			else runtime.previewTabs.navigate(target, tabId)
			return
		}
		runtime.previewTabs.open({
			type: 'page',
			href: `${base}${clean}${query.size ? `?${query}` : ''}`,
			label: previewLocationLabel(clean)
		})
	}

	// Every in-app link under the tab, not only the ones that call `onNavigate`: a plain anchor
	// followed by the browser would take it out of the session. `container` is the tab's host,
	// which also holds the drawers the page portals out of this component. Listened for on
	// `body`, between the link's own handlers (Svelte delegates them to the app root inside it)
	// and SvelteKit's router (on the document element), so it leaves alone a click a handler
	// already took and beats the router to the rest. A `#` link is a control's own; followed,
	// it would write into the sessions page's hash.
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
			onNavigate(`${url.pathname}${url.search}`)
		}
		document.body.addEventListener('click', onClick)
		return () => document.body.removeEventListener('click', onClick)
	})
</script>

<!-- Remount to refetch: both detail components load on mount, so a deploy lands here
     rather than as a refresh path of their own. -->
	{#key loadKey}
		{@const mountedKey = loadKey}
		{@const setLoadState = (state: 'loaded' | 'not_found') => onLoadState(mountedKey, state)}
		{#if notDeployed}
			<!-- This side shows the DEPLOYED item, and there isn't one. Without this the detail
		     page renders nothing and the panel is simply blank, which reads as a failure
		     rather than as the ordinary state of a draft nobody has deployed yet. -->
			<div class="flex-1 flex flex-col items-center justify-center gap-3 p-8 text-center">
				<div class="text-sm text-primary">Not deployed yet</div>
				<div class="text-xs text-tertiary max-w-sm">
					Nothing is deployed at <span class="font-mono">{path}</span>, so there is no deployed
					version to view. Deploy it from the editor to see it here.
				</div>
				<Button unifiedSize="sm" variant="default" startIcon={{ icon: Pen }} on:click={toEditSide}>
					Back to editor
				</Button>
			</div>
		{:else if kind === 'script'}
			<ScriptDetail
				hash={path}
				workspace={workspaceId}
				{searchParams}
				{onNavigate}
				{active}
				embedded
				onLoadState={setLoadState}
			/>
		{:else if kind === 'flow'}
			<FlowDetail
				{path}
				workspace={workspaceId}
				{searchParams}
				{onNavigate}
				{active}
				embedded
				onLoadState={setLoadState}
			/>
		{:else}
			<InWorkspaceAppViewer
				workspace={workspaceId}
				{path}
				onEdit={() => onNavigate(`/apps_raw/edit/${path}`)}
				onLoadState={setLoadState}
				syncHashToUrl={false}
			/>
		{/if}
	{/key}
