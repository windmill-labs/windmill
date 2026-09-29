<script lang="ts">
	import { setContext, untrack } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { setOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { setHostedPage, type HostedPage } from '$lib/components/hostedPage'
	import { pageItemForListPath, parsePreviewItemRoute } from './previewPaths'
	import { previewLocationLabel } from './previewRouter'
	import { whereIs } from './sessionPreviewTabs.svelte'
	import type { SessionPreviewTab } from './sessionState.svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'

	let {
		runtime,
		tab,
		path,
		workspaceId,
		container,
		reloadNonce = 0
	}: {
		runtime: SessionRuntime
		tab: SessionPreviewTab
		/** The list page, base-stripped. */
		path: string
		workspaceId: string
		/** The tab's element, which also holds the list's portalled drawers and menus. */
		container: HTMLElement
		/** Bumped to load the list again from the server. */
		reloadNonce?: number
	} = $props()

	// Mounted in the preview panel, outside the chat's own subtree, as PageItemEditorView is.
	setContext(
		'aiChatManager',
		untrack(() => runtime.manager)
	)
	setOperatingWorkspace(() => workspaceId)

	// The tab is where the list keeps its filters: written as the tab's observed location, so
	// the chat reads them and a remount restores them, as it would for a frame.
	const locationParts = () => {
		const loc = whereIs(tab).split('#')[0]
		const at = loc.indexOf('?')
		return at < 0
			? { pathname: loc, search: '' }
			: { pathname: loc.slice(0, at), search: loc.slice(at) }
	}
	const host: HostedPage = {
		get search() {
			return locationParts().search
		},
		setSearch(search) {
			runtime.previewTabs.observeLocation(tab.id, locationParts().pathname + search)
		},
		openItem(itemPath) {
			const ref = pageItemForListPath(path, itemPath)
			if (ref) runtime.previewTabs.open({ type: 'pageitem', ref })
		},
		openLink(href) {
			const item = parsePreviewItemRoute(href)
			if (item) {
				runtime.previewTabs.open({
					type: 'item',
					item: { kind: item.kind, raw_app: item.raw_app, path: item.itemPath, summary: '' }
				})
			} else {
				runtime.previewTabs.open({ type: 'page', href, label: previewLocationLabel(href) })
			}
		}
	}
	setHostedPage(host)

	// Any in-app link under the tab opens in the session rather than taking the whole window off
	// it: a list, its drawers and its menus carry plain hrefs throughout (runs, audit logs, a
	// job). A `#` link is a row's own: its handler opens the row through the host, and following
	// it would write the row into the sessions page's own hash.
	$effect(() => {
		const el = container
		function onClick(e: MouseEvent) {
			if (e.defaultPrevented || e.button !== 0) return
			if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return
			const a = (e.target as Element | null)?.closest?.('a[href]') as HTMLAnchorElement | null
			if (!a || (a.target && a.target !== '_self') || a.hasAttribute('download')) return
			const url = new URL(a.href, window.location.href)
			if (url.origin !== window.location.origin) return
			e.preventDefault()
			if (a.getAttribute('href')?.startsWith('#')) return
			host.openLink(url.pathname + url.search + url.hash)
		}
		el.addEventListener('click', onClick)
		return () => el.removeEventListener('click', onClick)
	})

	const listed = $derived(pageItemForListPath(path, ''))
</script>

{#snippet loading()}
	<div class="flex-1 flex items-center justify-center text-tertiary">
		<Loader2 class="animate-spin" />
	</div>
{/snippet}

<!-- Dynamic imports, as the editors: each list pulls in its item's editor. -->
<div class="flex h-full min-h-0 flex-col overflow-auto">
	{#key `${path}:${workspaceId}:${reloadNonce}`}
		{#if listed?.kind === 'variable'}
			{#await import('$lib/components/variables/VariablesList.svelte')}
				{@render loading()}
			{:then Module}
				<Module.default />
			{/await}
		{:else if listed?.kind === 'resource'}
			{#await import('$lib/components/resources/ResourcesList.svelte')}
				{@render loading()}
			{:then Module}
				<Module.default />
			{/await}
		{:else if listed?.kind === 'schedule'}
			{#await import('$lib/components/schedules/SchedulesList.svelte')}
				{@render loading()}
			{:then Module}
				<Module.default />
			{/await}
		{:else if listed?.kind === 'trigger'}
			{#await import('$lib/components/triggers/TriggerList.svelte')}
				{@render loading()}
			{:then Module}
				<Module.default triggerKind={listed.triggerKind} />
			{/await}
		{/if}
	{/key}
</div>
