<!--
@component
Workspace drill picker — adapter over the generic `DrillPicker`. Preserves
the workspace-specific public API (kinds, scope = `{ kind, dir? }`,
currentItem, leaf/branch icons) so callers (BreadcrumbSegment, EditorHeader)
don't need to know about the generic tree model underneath.

Surfaces AI-created localStorage drafts (via `listGlobalDrafts`) as extra
items alongside the backend-loaded list, so chat-scaffolded scripts/flows/
apps that haven't been deployed yet are still navigable. Gated on
`isGlobalAiEnabled()` — without sessions, the only UserDrafts present are
standalone editor autosaves and surfacing those in the breadcrumb picker
would be surprising.
-->
<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { untrack } from 'svelte'
	import {
		getCachedItems,
		loadKind,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from './workspacePicker'
	import DrillPicker from './DrillPicker.svelte'
	import type { DrillBranch, DrillLeaf } from './drillPicker'
	import { buildWorkspaceTree, legacyScopeToPath, relativizeWorkspacePath } from './workspaceTree'
	import { listGlobalDrafts } from '$lib/components/copilot/chat/global/userDraftAdapter'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'

	type Kind = WorkspaceItemKind
	type ScopeKind = Kind | 'all'

	export type Scope = { kind: ScopeKind; dir?: string } | undefined

	interface Props {
		onPick: (item: WorkspaceItem) => void
		kinds?: Kind[]
		initialScope?: Scope
		initialHighlight?: string
		currentItem?: WorkspaceItem & { savedPath?: string }
		externalFilter?: string
		autoFocus?: boolean
		flush?: boolean
	}

	let {
		onPick,
		kinds = ['flow', 'script', 'app'],
		initialScope,
		initialHighlight,
		currentItem,
		externalFilter,
		autoFocus = true,
		flush = false
	}: Props = $props()

	let inner = $state<DrillPicker<WorkspaceItem> | undefined>(undefined)

	export function focus() {
		inner?.focus()
	}
	export function handleKeydown(e: KeyboardEvent) {
		inner?.handleKeydown(e)
	}
	export function pickHighlighted() {
		inner?.pickHighlighted()
	}

	// Seed from the last fetched snapshot so kinds already fetched in this
	// session render on the first frame. Each entry is replaced once
	// `loadKind` returns fresh data — stale-while-revalidate, so deploys and
	// AI-created drafts surface on the next open without explicit cache
	// busting.
	let loaded = $state<Partial<Record<Kind, WorkspaceItem[]>>>(
		(() => {
			if (!$workspaceStore) return {}
			const out: Partial<Record<Kind, WorkspaceItem[]>> = {}
			for (const k of kinds) {
				const cached = getCachedItems($workspaceStore, k)
				if (cached) out[k] = cached
			}
			return out
		})()
	)
	let loadingKind = $state<Partial<Record<Kind, boolean>>>({})

	async function ensureLoaded(kind: Kind) {
		if (!$workspaceStore) return
		// Always re-fetch. If we have nothing cached, show a spinner; if we do,
		// keep displaying it and quietly swap to fresh data when it lands.
		// `loaded[kind]` is read inside `untrack(...)` because this function is
		// reachable from the search `$effect` below — without the untrack,
		// that effect would subscribe to the signal `ensureLoaded` fills, and
		// each `loaded[kind] = items` (proxy `set` notifies even when the ref
		// is unchanged from cache) would refire it → runaway loop. Drill
		// navigation goes through `handleScopeChange` so it's a callback
		// reaction to user navigation, never a reactive consequence.
		if (!untrack(() => loaded[kind])) loadingKind[kind] = true
		try {
			const items = await loadKind($workspaceStore, kind)
			loaded[kind] = items
		} finally {
			loadingKind[kind] = false
		}
	}

	// Chat tools and session editor previews write drafts through `UserDraft`
	// (workspace-scoped, localStorage-backed). Merge those into the picker so
	// users can navigate to in-flight items that haven't been deployed yet.
	// Filter to kinds the picker actually displays. Gated on the global-AI
	// flag — without sessions, the only UserDrafts present are the standalone
	// editors' autosaves and surfacing those in the breadcrumb picker would
	// be surprising (they'd appear as navigable items that 404 on the backend
	// draft fetch).
	const KIND_TO_DRAFT_TYPE = { flow: 'flow', script: 'script', app: 'app' } as const
	function aiDraftsForKind(k: Kind): WorkspaceItem[] {
		if (!isGlobalAiEnabled()) return []
		if (!$workspaceStore) return []
		const targetType = KIND_TO_DRAFT_TYPE[k]
		return listGlobalDrafts($workspaceStore)
			.filter((d) => d.type === targetType)
			.map((d) => ({
				path: d.path,
				summary: d.summary ?? '',
				kind: k,
				// `raw_app` lives on the draft envelope for legacy/raw-app distinction.
				raw_app: k === 'app' ? !!(d.value as { files?: unknown })?.files : undefined
			}))
	}

	const extraItemsByKind = $derived<Partial<Record<Kind, WorkspaceItem[]>>>(
		Object.fromEntries(kinds.map((k) => [k, aiDraftsForKind(k)]))
	)

	const tree = $derived(
		buildWorkspaceTree({ loaded, kinds, currentItem, loadingKind, extraItemsByKind })
	)

	// Mount-time only: callers (BreadcrumbSegment, EditorHeader) snapshot the
	// scope when the popover opens, so re-evaluating on prop changes would
	// fight the user's drilling.
	const computedInitialScope = untrack(() => legacyScopeToPath(initialScope, kinds))

	// Triggered on scope change: load the kind the user just drilled into.
	// `'all'` triggers loads for every kind since it merges across them.
	function handleScopeChange(scope: string[]) {
		if (scope.length === 0) return
		const top = scope[0]
		// top is either `kind:<k>` or (when kinds.length===1) `dir:<k>:<path>`
		if (top.startsWith('kind:')) {
			const k = top.slice(5) as ScopeKind
			if (k === 'all') for (const x of kinds) ensureLoaded(x)
			else if (kinds.includes(k as Kind)) ensureLoaded(k as Kind)
		} else if (top.startsWith('dir:')) {
			// Single-kind mode: dir:<k>:<path>
			const rest = top.slice(4)
			const colon = rest.indexOf(':')
			if (colon > 0) {
				const k = rest.slice(0, colon) as ScopeKind
				if (k === 'all') for (const x of kinds) ensureLoaded(x)
				else if (kinds.includes(k as Kind)) ensureLoaded(k as Kind)
			}
		}
	}

	// Search is global → load every kind. In external-search mode the host
	// drives the filter, so we react to it. In internal-search mode (no
	// externalFilter, DrillPicker renders its own search box) we can't see
	// the filter from here, so eagerly preload on mount instead — the cached
	// snapshot in `loaded` makes this near-instant on warm sessions.
	$effect(() => {
		if (externalFilter === undefined) {
			for (const k of kinds) ensureLoaded(k)
		} else if (externalFilter.trim() !== '') {
			for (const k of kinds) ensureLoaded(k)
		}
	})
</script>

{#snippet leafIcon(leaf: DrillLeaf<WorkspaceItem>)}
	<RowIcon kind={leaf.data.kind} size={12} />
{/snippet}

{#snippet branchIcon(branch: DrillBranch<WorkspaceItem>)}
	{#if branch.key === 'kind:flow' || branch.key === 'kind:script' || branch.key === 'kind:app'}
		{@const k = branch.key.slice(5) as Kind}
		<RowIcon kind={k} size={12} />
	{:else if branch.icon}
		{@const Icon = branch.icon}
		<Icon size={12} class="shrink-0 text-tertiary" />
	{/if}
{/snippet}

<DrillPicker
	bind:this={inner}
	{tree}
	onPick={(leaf) => onPick(leaf.data)}
	initialScope={computedInitialScope}
	{initialHighlight}
	{externalFilter}
	{autoFocus}
	{flush}
	{leafIcon}
	{branchIcon}
	leafSecondary={(leaf, scope) => relativizeWorkspacePath(leaf.data.path, scope)}
	onScopeChange={handleScopeChange}
/>
