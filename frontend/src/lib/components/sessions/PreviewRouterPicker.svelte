<!--
@component
Preview "router" picker — a `WorkspaceItemDrillPicker` superset used by the
session preview breadcrumb. Same drill model and workspace-item branches, plus
a leading "Pages" branch (Home, Runs, Workspace settings, …) so the breadcrumb
can route the preview iframe to non-item pages too, not just scripts/flows/apps.

Kept separate from `WorkspaceItemDrillPicker` so the editor breadcrumb (which
only ever navigates between items) doesn't grow a Pages section.
-->
<script lang="ts" module>
	export type Scope = { kind: 'flow' | 'script' | 'app' | 'all'; dir?: string } | undefined
</script>

<script lang="ts">
	import { untrack } from 'svelte'
	import { Compass, FileText } from 'lucide-svelte'
	import { resource } from 'runed'
	import { workspaceStore } from '$lib/stores'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { type WorkspaceItem, type WorkspaceItemKind } from '$lib/components/workspacePicker'
	import { useWorkspaceItemsLoader } from '$lib/components/workspaceItemsLoader.svelte'
	import DrillPicker from '../DrillPicker.svelte'
	import type { DrillBranch, DrillLeaf, DrillNode } from '$lib/components/drillPicker'
	import {
		buildWorkspaceTree,
		legacyScopeToPath,
		relativizeWorkspacePath
	} from '$lib/components/workspaceTree'
	import { listGlobalDrafts } from '$lib/components/copilot/chat/global/userDraftAdapter'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import type { PersistedArtifact } from '$lib/components/copilot/chat/artifacts/artifactsDB'
	import {
		PREVIEW_PAGES,
		artifactKey,
		isArtifactKey,
		pageHref,
		pageKey,
		type PreviewTarget
	} from './previewRouter'

	type Kind = WorkspaceItemKind
	type DrillPickerHandle = {
		focus: () => void
		handleKeydown: (e: KeyboardEvent) => void
		pickHighlighted: () => void
	}

	interface Props {
		onPick: (target: PreviewTarget) => void
		initialScope?: Scope
		initialHighlight?: string
		currentItem?: WorkspaceItem & { savedPath?: string }
		externalFilter?: string
		autoFocus?: boolean
		flush?: boolean
		// The session's effective workspace. Items and drafts must load from it,
		// not the navigation workspace ($workspaceStore) — a fork-scoped session
		// leaves the global store on the nav workspace, so keying off it would
		// list root items and miss fork-only ones. Falls back to $workspaceStore
		// for non-session consumers.
		workspaceId?: string
		// Session artifacts to surface as an "Artifacts" branch; omitted or empty
		// hides the branch (non-session consumers, sessions without artifacts).
		artifacts?: PersistedArtifact[]
	}

	let {
		onPick,
		initialScope,
		initialHighlight,
		currentItem,
		externalFilter,
		autoFocus = true,
		flush = false,
		workspaceId,
		artifacts
	}: Props = $props()

	const kinds: Kind[] = ['flow', 'script', 'app']
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

	// Surface AI-created drafts the same way the editor picker does so in-flight
	// chat-scaffolded items are navigable (see WorkspaceItemDrillPicker).
	const KIND_TO_DRAFT_TYPE = { flow: 'flow', script: 'script', app: 'app' } as const
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
				raw_app: k === 'app' ? !!(d.value as { files?: unknown })?.files : undefined
			}))
	}
	const extraItemsByKind = $derived<Partial<Record<Kind, WorkspaceItem[]>>>(
		Object.fromEntries(kinds.map((k) => [k, aiDraftsForKind(k)]))
	)

	// Wrap item leaves as PreviewTargets so the tree can carry page leaves too.
	// Branch structure is untouched — only leaf `data` is re-tagged.
	function tagItems(nodes: DrillNode<WorkspaceItem>[]): DrillNode<PreviewTarget>[] {
		return nodes.map((n) =>
			n.type === 'leaf'
				? { ...n, data: { type: 'item', item: n.data } }
				: { ...n, children: tagItems(n.children) }
		)
	}

	function pageLeaf(p: (typeof PREVIEW_PAGES)[number]): DrillLeaf<PreviewTarget> {
		return {
			type: 'leaf',
			key: pageKey(p.path),
			label: p.label,
			icon: p.icon,
			data: { type: 'page', href: pageHref(p.path), label: p.label }
		}
	}

	// Home is deliberately not navigable from the session preview — it stays in
	// PREVIEW_PAGES only so a tab already sitting on '/' keeps its label.
	const pagesBranch = $derived<DrillBranch<PreviewTarget>>({
		type: 'branch',
		key: 'pages',
		label: 'Pages',
		icon: Compass,
		searchGroup: true,
		children: PREVIEW_PAGES.filter((p) => p.path !== '/').map(pageLeaf)
	})

	// Session artifacts, when present, get their own branch between Pages and the
	// workspace items so they're pickable (and searchable) like any destination.
	const artifactsBranch = $derived<DrillBranch<PreviewTarget> | undefined>(
		artifacts && artifacts.length > 0
			? {
					type: 'branch',
					key: 'artifacts',
					label: 'Artifacts',
					icon: FileText,
					searchGroup: true,
					children: artifacts.map((a) => ({
						type: 'leaf' as const,
						key: artifactKey(a.id),
						label: a.name,
						icon: FileText,
						data: { type: 'artifact', id: a.id, name: a.name }
					}))
				}
			: undefined
	)

	const tree = $derived<DrillNode<PreviewTarget>[]>([
		pagesBranch,
		...(artifactsBranch ? [artifactsBranch] : []),
		...tagItems(
			buildWorkspaceTree({
				loaded: loader.loaded,
				kinds,
				currentItem,
				loadingKind: loader.loadingKind,
				extraItemsByKind
			})
		)
	])

	// An artifact highlight lives under the 'artifacts' branch, which the legacy
	// {kind, dir} scope can't express — open the picker inside that branch so the
	// active artifact is actually visible and highlighted (not the first root row).
	// Keyed on the highlight's shape alone: the branch itself may not have
	// materialized yet (artifacts hydrate from IndexedDB after tabs restore), and
	// the drill entries fill in reactively once it does.
	const computedInitialScope = untrack(() =>
		initialHighlight && isArtifactKey(initialHighlight)
			? ['artifacts']
			: legacyScopeToPath(initialScope, kinds)
	)
</script>

{#snippet leafIcon(leaf: DrillLeaf<PreviewTarget>)}
	{#if leaf.data.type === 'item'}
		<RowIcon kind={leaf.data.item.kind} size={12} />
	{:else if leaf.icon}
		{@const Icon = leaf.icon}
		<Icon size={12} class="shrink-0 text-tertiary" />
	{/if}
{/snippet}

{#snippet branchIcon(branch: DrillBranch<PreviewTarget>)}
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
	leafSecondary={(leaf, scope) =>
		leaf.data.type === 'item' ? relativizeWorkspacePath(leaf.data.item.path, scope) : undefined}
	onScopeChange={(scope) => {
		if (scope.length > 0) loader.ensureForScopeSegment(scope[0])
	}}
	onFilterChange={loader.onFilterChange}
/>
