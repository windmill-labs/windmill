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
	import { type WorkspaceItem, type WorkspaceItemKind } from './workspacePicker'
	import { useWorkspaceItemsLoader } from './workspaceItemsLoader.svelte'
	import DrillPicker from './DrillPicker.svelte'
	import type { DrillBranch, DrillLeaf } from './drillPicker'
	import { buildWorkspaceTree, legacyScopeToPath, relativizeWorkspacePath } from './workspaceTree'
	import { listGlobalDrafts } from '$lib/components/copilot/chat/global/userDraftAdapter'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { resource } from 'runed'

	type Kind = WorkspaceItemKind
	type ScopeKind = Kind | 'all'
	type DrillPickerHandle = {
		focus: () => void
		handleKeydown: (e: KeyboardEvent) => void
		pickHighlighted: () => void
	}

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
		// Load items and drafts from this workspace instead of the navigation
		// workspace. Set by session live editors, whose acting workspace can
		// differ from $workspaceStore; falls back to $workspaceStore otherwise.
		workspaceId?: string
	}

	let {
		onPick,
		kinds = ['flow', 'script', 'app'],
		initialScope,
		initialHighlight,
		currentItem,
		externalFilter,
		autoFocus = true,
		flush = false,
		workspaceId
	}: Props = $props()

	const effectiveWorkspace = $derived(workspaceId ?? $workspaceStore)

	let inner = $state<DrillPickerHandle | undefined>(undefined)

	export function focus() {
		inner?.focus()
	}
	export function handleKeydown(e: KeyboardEvent) {
		inner?.handleKeydown(e)
	}
	export function pickHighlighted() {
		inner?.pickHighlighted()
	}

	const loader = useWorkspaceItemsLoader(
		() => effectiveWorkspace,
		() => kinds
	)

	// Chat tools and session editor previews write drafts through `UserDraft`
	// (workspace-scoped, localStorage-backed). Merge those into the picker so
	// users can navigate to in-flight items that haven't been deployed yet.
	// Filter to kinds the picker actually displays. Gated on the global-AI
	// flag — without sessions, the only UserDrafts present are the standalone
	// editors' autosaves and surfacing those in the breadcrumb picker would
	// be surprising (they'd appear as navigable items that 404 on the backend
	// draft fetch).
	const KIND_TO_DRAFT_TYPE = { flow: 'flow', script: 'script', app: 'app' } as const
	// `listGlobalDrafts` is backend-backed (async); fetch once and derive the
	// per-kind lists synchronously from the resolved snapshot.
	const globalDraftsResource = resource(
		() => ({ ws: effectiveWorkspace, enabled: isGlobalAiEnabled() }),
		async ({ ws, enabled }) => (enabled && ws ? await listGlobalDrafts(ws) : [])
	)
	function aiDraftsForKind(k: Kind): WorkspaceItem[] {
		const targetType = KIND_TO_DRAFT_TYPE[k]
		return (globalDraftsResource.current ?? [])
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
		buildWorkspaceTree({
			loaded: loader.loaded,
			kinds,
			currentItem,
			loadingKind: loader.loadingKind,
			extraItemsByKind
		})
	)

	// Mount-time only: callers (BreadcrumbSegment, EditorHeader) snapshot the
	// scope when the popover opens, so re-evaluating on prop changes would
	// fight the user's drilling.
	const computedInitialScope = untrack(() => legacyScopeToPath(initialScope, kinds))
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
	onScopeChange={(scope) => {
		if (scope.length > 0) loader.ensureForScopeSegment(scope[0])
		// Single-kind layout has no kind branch at root — `buildWorkspaceTree`
		// collapses to the kind's children. The picker mounts with scope=[],
		// so without this fallback nothing fires until the user searches.
		else if (kinds.length === 1) loader.ensureLoaded(kinds[0])
	}}
	onFilterChange={loader.onFilterChange}
/>
