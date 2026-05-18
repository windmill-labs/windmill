<!--
@component
Workspace drill picker — adapter over the generic `DrillPicker`. Preserves
the workspace-specific public API (kinds, scope = `{ kind, dir? }`,
currentItem, leaf/branch icons) so callers (BreadcrumbSegment, EditorHeader)
don't need to know about the generic tree model underneath.
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

	// Seed from module-level cache so kinds already fetched in this session
	// render on the first frame. Read once at mount: melt-ui mounts a fresh
	// picker per popover open, so workspace changes are picked up at the
	// next open without needing this seed to be reactive.
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
		if (loaded[kind]) return
		loadingKind[kind] = true
		try {
			const items = await loadKind($workspaceStore, kind)
			loaded[kind] = items
		} finally {
			loadingKind[kind] = false
		}
	}

	const tree = $derived(buildWorkspaceTree({ loaded, kinds, currentItem, loadingKind }))

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

	// Search is global → load every kind.
	$effect(() => {
		if ((externalFilter ?? '').trim() !== '') {
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
