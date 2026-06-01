<!--
@component
AI chat `@`-mention dropdown. Mounts the generic `DrillPicker` with a
unified tree:

  Diffs / Modules / Databases / Workspace
                                     ├── All / Flows / Scripts
                                     │       └── f/scope/sub/leaf …

The Diffs / Modules / Databases branches are synthesized from the chat's
in-memory `availableContext`. The Workspace branch delegates to
`buildWorkspaceTree` so the picker shares the workspace caching machinery
with the standalone picker used by `EditorHeader`.

On a workspace-leaf pick, emits a reference-only `WorkspaceScriptElement` /
`WorkspaceFlowElement` (path + title + summary). Content is materialized
at message-prep time by `AIChatManager` — see PR #9216.
-->
<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { Database, Diff, Layers } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import type { FlowModule } from '$lib/gen/types.gen'
	import DrillPicker from '$lib/components/DrillPicker.svelte'
	import type { DrillBranch, DrillLeaf, DrillNode } from '$lib/components/drillPicker'
	import {
		getCachedItems,
		loadKind,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/workspacePicker'
	import { buildWorkspaceTree, relativizeWorkspacePath } from '$lib/components/workspaceTree'
	import {
		ContextIconMap,
		type ContextElement,
		type WorkspaceFlowElement,
		type WorkspaceScriptElement
	} from './context'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		onSelect: (element: ContextElement) => void
		onSelectWorkspaceItem?: (element: ContextElement) => void
		setShowing?: (showing: boolean) => void
		externalFilter?: string
		autoFocus?: boolean
	}

	let {
		availableContext,
		selectedContext,
		onSelect,
		onSelectWorkspaceItem,
		setShowing,
		externalFilter,
		autoFocus = true
	}: Props = $props()

	// Chat tree leaves carry either a workspace path (resolved to content
	// at pick time) or a runtime ContextElement (added directly).
	type ChatLeafData = WorkspaceItem | ContextElement

	let inner = $state<DrillPicker | undefined>(undefined)

	export function handleKeydown(e: KeyboardEvent) {
		inner?.handleKeydown(e)
	}

	// Hide already-selected runtime context in the badge popover (no
	// external filter). Keep them in the inline-search view so a re-match
	// isn't suppressed (matches the prior `showAllAvailable={true}` path).
	const hideSelected = $derived(externalFilter === undefined)

	function isSelected(c: ContextElement): boolean {
		return selectedContext.some((s) => s.type === c.type && s.title === c.title)
	}

	// Workspace state — same shape as WorkspaceItemDrillPicker. Seed from
	// the module-level cache; load on scope entry or on global search.
	const WORKSPACE_KINDS: WorkspaceItemKind[] = ['flow', 'script']
	let loaded = $state<Partial<Record<WorkspaceItemKind, WorkspaceItem[]>>>(
		(() => {
			if (!$workspaceStore) return {}
			const out: Partial<Record<WorkspaceItemKind, WorkspaceItem[]>> = {}
			for (const k of WORKSPACE_KINDS) {
				const cached = getCachedItems($workspaceStore, k)
				if (cached) out[k] = cached
			}
			return out
		})()
	)
	let loadingKind = $state<Partial<Record<WorkspaceItemKind, boolean>>>({})

	async function ensureLoaded(kind: WorkspaceItemKind) {
		if (!$workspaceStore) return
		if (loaded[kind]) return
		loadingKind[kind] = true
		try {
			const items = await loadKind($workspaceStore, kind)
			loaded[kind] = items
		} finally {
			loadingKind[kind] = false
		}
	}

	function contextLeaf(c: ContextElement): DrillLeaf<ChatLeafData> {
		const displayLabel =
			c.type === 'diff' || c.type === 'flow_module' ? c.title.replace(/_/g, ' ') : c.title
		return {
			type: 'leaf',
			key: `${c.type}:${c.title}`,
			label: displayLabel,
			// Keep the raw title (with underscores) in the search haystack so
			// `@my_module` matches as well as the display form `my module`.
			// Skip the join when the display form is the raw title itself
			// (e.g. `db` elements) to avoid `"x x"` haystacks.
			searchableText: displayLabel === c.title ? c.title : `${displayLabel} ${c.title}`,
			data: c
		}
	}

	function buildContextBranch(
		id: 'diffs' | 'modules' | 'databases',
		label: string,
		icon: any,
		type: 'diff' | 'flow_module' | 'db'
	): DrillBranch<ChatLeafData> | null {
		const filtered = availableContext.filter(
			(c) => c.type === type && (!hideSelected || !isSelected(c))
		)
		if (filtered.length === 0) return null
		return {
			type: 'branch',
			key: id,
			label,
			icon,
			searchGroup: true,
			children: filtered.map(contextLeaf)
		}
	}

	const tree = $derived<DrillNode<ChatLeafData>[]>(
		(() => {
			const branches: DrillNode<ChatLeafData>[] = []
			const diffs = buildContextBranch('diffs', 'Diffs', Diff, 'diff')
			const modules = buildContextBranch('modules', 'Modules', BarsStaggered, 'flow_module')
			const dbs = buildContextBranch('databases', 'Databases', Database, 'db')
			if (diffs) branches.push(diffs)
			if (modules) branches.push(modules)
			if (dbs) branches.push(dbs)
			const wsChildren = buildWorkspaceTree({
				loaded,
				kinds: WORKSPACE_KINDS,
				loadingKind
			}) as DrillNode<ChatLeafData>[]
			// Workspace-only (e.g. global chat with no diffs/modules/dbs): skip
			// the redundant 'Workspace' row and surface its children at the root.
			if (branches.length === 0) return wsChildren
			branches.push({
				type: 'branch',
				key: 'workspace',
				label: 'Workspace',
				icon: Layers,
				children: wsChildren
			})
			return branches
		})()
	)

	function handlePick(leaf: DrillLeaf<ChatLeafData>) {
		const d = leaf.data
		if ('kind' in d) {
			// Workspace item — emit a reference-only workspace_* element.
			// Content is fetched at message-prep time by the chat manager
			// (see PR #9216 which switched workspace context to references).
			if (!onSelectWorkspaceItem) return
			if (d.kind === 'script') {
				const element: WorkspaceScriptElement & { deletable: boolean } = {
					type: 'workspace_script',
					path: d.path,
					title: d.path,
					summary: d.summary,
					deletable: true
				}
				onSelectWorkspaceItem(element)
			} else if (d.kind === 'flow') {
				const element: WorkspaceFlowElement & { deletable: boolean } = {
					type: 'workspace_flow',
					path: d.path,
					title: d.path,
					summary: d.summary,
					deletable: true
				}
				onSelectWorkspaceItem(element)
			}
			// Apps are filtered out via kinds=['flow','script']; ignore.
		} else {
			// Runtime context element — added directly.
			onSelect(d)
		}
	}

	function handleScopeChange(scope: string[]) {
		// Two possible layouts:
		//   (a) WRAPPED   — `['workspace', 'kind:all', ...]` — chat with Diffs /
		//       Modules / Databases branches alongside Workspace.
		//   (b) UNWRAPPED — `['kind:all', ...]` or `['dir:flow:...']` — chat
		//       with only the workspace branch (global chat). The redundant
		//       'workspace' wrapper is collapsed in the tree builder.
		// Strip the wrapper so the rest of this function only deals with the
		// inner workspace path.
		const inWorkspace = scope.length === 0 || scope[0] === 'workspace'
		const path = scope[0] === 'workspace' ? scope.slice(1) : scope
		// 'diffs' / 'modules' / 'databases' are synthesised — no fetch.
		if (!inWorkspace) return
		// Entering the Workspace branch (or its 'All' sub-branch): preload
		// every kind eagerly. Without this, the user lands on a screen of
		// kind branches whose `loaded[k]` is undefined and `loadingKind[k]`
		// is also undefined, so `buildWorkspaceTree` reports neither items
		// nor a spinner — looks empty until they drill into a single kind.
		if (path.length === 0 || path[0] === 'kind:all') {
			for (const k of WORKSPACE_KINDS) ensureLoaded(k)
			return
		}
		if (path[0].startsWith('kind:')) {
			const k = path[0].slice(5) as WorkspaceItemKind
			if (WORKSPACE_KINDS.includes(k)) ensureLoaded(k)
		} else if (path[0].startsWith('dir:')) {
			// Unwrapped + single-kind: top scope is `dir:<kind>:<path>` directly.
			// (In the wrapped layout the kind branch always precedes `dir:`.)
			const rest = path[0].slice(4)
			const colon = rest.indexOf(':')
			if (colon > 0) {
				const k = rest.slice(0, colon) as WorkspaceItemKind
				if (WORKSPACE_KINDS.includes(k)) ensureLoaded(k)
			}
		}
	}

	// Global search loads every workspace kind so results appear across
	// the tree.
	$effect(() => {
		if ((externalFilter ?? '').trim() !== '') {
			for (const k of WORKSPACE_KINDS) ensureLoaded(k)
		}
	})

	// Close the picker on Escape. The badge popover's melt-ui handles Esc
	// itself; for the inline-mention case (Portal-rendered, no melt) we
	// listen at the document level. Skip when an upstream handler already
	// claimed the event (defensive — melt-ui doesn't currently
	// preventDefault on Esc, but a future host might).
	function onDocumentKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !e.defaultPrevented) {
			setShowing?.(false)
		}
	}
	$effect(() => {
		document.addEventListener('keydown', onDocumentKeydown)
		return () => document.removeEventListener('keydown', onDocumentKeydown)
	})
</script>

{#snippet leafIcon(leaf: DrillLeaf<ChatLeafData>)}
	{@const d = leaf.data}
	{#if 'kind' in d}
		<RowIcon kind={d.kind} size={12} />
	{:else if d.type === 'flow_module'}
		<FlowModuleIcon module={d as unknown as FlowModule} size={14} />
	{:else}
		{@const Icon = ContextIconMap[d.type]}
		{#if Icon}<Icon size={12} class="shrink-0" />{/if}
	{/if}
{/snippet}

{#snippet branchIcon(branch: DrillBranch<ChatLeafData>)}
	{#if branch.key === 'kind:flow' || branch.key === 'kind:script' || branch.key === 'kind:app'}
		{@const k = branch.key.slice(5) as WorkspaceItemKind}
		<RowIcon kind={k} size={12} />
	{:else if branch.icon}
		{@const Icon = branch.icon}
		<Icon size={12} class="shrink-0 text-tertiary" />
	{/if}
{/snippet}

<DrillPicker
	bind:this={inner}
	{tree}
	onPick={handlePick}
	{externalFilter}
	{autoFocus}
	{leafIcon}
	{branchIcon}
	leafSecondary={(leaf, scope) =>
		'kind' in leaf.data ? relativizeWorkspacePath(leaf.data.path, scope) : undefined}
	onScopeChange={handleScopeChange}
/>
