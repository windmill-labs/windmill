<!--
@component
AI chat `@`-mention dropdown. Mounts the generic `DrillPicker` with a
unified tree:

  Diffs / Modules / Databases / Workspace
                                     ├── u/user / f/folder …
                                     │       └── sub/leaf … (kinds mixed)

The Diffs / Modules / Databases branches are synthesized from the chat's
in-memory `availableContext`. The Workspace branch delegates to
`buildWorkspaceTree` (flat layout — the workspace home's first level, no
kind grouping) so the picker shares the workspace caching machinery with
the standalone picker used by `EditorHeader`. Apps only appear in GLOBAL
chat and list raw (code-based) apps; visual apps are excluded.

On a workspace-leaf pick, emits a reference-only `WorkspaceScriptElement` /
`WorkspaceFlowElement` / `WorkspaceAppElement` (path + title + summary).
Content is materialized at message-prep time by `AIChatManager` — see PR #9216.
-->
<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { Database, Diff, FileText, Folder, Layers } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import type { FlowModule } from '$lib/gen/types.gen'
	import DrillPicker from '$lib/components/DrillPicker.svelte'
	import type { DrillBranch, DrillIcon, DrillLeaf, DrillNode } from '$lib/components/drillPicker'
	import {
		workspaceItemDisplayPath,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/workspacePicker'
	import { useWorkspaceItemsLoader } from '$lib/components/workspaceItemsLoader.svelte'
	import { buildWorkspaceTree, relativizeWorkspacePath } from '$lib/components/workspaceTree'
	import { getFileIcon } from '$lib/components/icons/fileIcon'
	import { getAiChatManager } from './aiChatManagerContext'
	import { AIMode } from './AIChatManager.svelte'
	import type { AttachedFile, AttachedFolder } from './files/attachedFiles.svelte'
	import {
		ContextIconMap,
		type ContextElement,
		type WorkspaceAppElement,
		type WorkspaceFlowElement,
		type WorkspaceScriptElement
	} from './context'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		onSelect: (element: ContextElement) => void
		onSelectWorkspaceItem?: (element: ContextElement) => void
		/** GLOBAL-chat only: pick an attached file to insert a plain `@filename`
		 * mention. When omitted, the Files branch is not surfaced. */
		onSelectFile?: (name: string) => void
		setShowing?: (showing: boolean) => void
		externalFilter?: string
		autoFocus?: boolean
	}

	let {
		availableContext,
		selectedContext,
		onSelect,
		onSelectWorkspaceItem,
		onSelectFile,
		setShowing,
		externalFilter,
		autoFocus = true
	}: Props = $props()

	const aiChatManager = getAiChatManager()

	// Chat tree leaves carry either a workspace path (resolved to content at
	// pick time), a runtime ContextElement (added directly), or an attached
	// file name (inserts an `@filename` mention).
	type FileLeafData = { fileName: string }
	type ChatLeafData = WorkspaceItem | ContextElement | FileLeafData
	type DrillPickerHandle = {
		handleKeydown: (e: KeyboardEvent) => void
	}

	let inner = $state<DrillPickerHandle | undefined>(undefined)

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

	// Workspace state shared with WorkspaceItemDrillPicker via the loader.
	// Flows and scripts everywhere; raw apps only in GLOBAL chat, where the
	// raw-app tool surface lets the model act on the reference. Visual apps stay
	// excluded — they're frontends, not code units (filtered out below, where the
	// 'app' kind is narrowed to raw apps before tree-building).
	const WORKSPACE_KINDS = $derived<WorkspaceItemKind[]>(
		aiChatManager.mode === AIMode.GLOBAL ? ['flow', 'script', 'app'] : ['flow', 'script']
	)
	const loader = useWorkspaceItemsLoader(
		() => $workspaceStore,
		() => WORKSPACE_KINDS
	)

	// The 'app' listing returns visual and raw apps together; keep only raw apps
	// so the picker never surfaces a frontend-only app that the chat can't use.
	const loadedForTree = $derived.by(() => {
		const loaded = loader.loaded
		if (!loaded.app) return loaded
		return { ...loaded, app: loaded.app.filter((a) => a.raw_app) }
	})

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
		icon: DrillIcon,
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

	// Attached files surface as a Files branch (GLOBAL mode only, and only when
	// the host wired `onSelectFile`). Picking one inserts an `@filename` mention
	// rather than adding a context element — the AI already has read/search tools.
	const attachedEnabled = $derived(!!onSelectFile && aiChatManager.mode === AIMode.GLOBAL)
	// Locked/unavailable folders have no pickable children — skip them here.
	const attachedFolders = $derived(
		attachedEnabled ? aiChatManager.attachedFiles.folders.filter((f) => f.files.length > 0) : []
	)
	const attachedStandalone = $derived(attachedEnabled ? aiChatManager.attachedFiles.standalone : [])
	const hasAttachments = $derived(attachedFolders.length > 0 || attachedStandalone.length > 0)

	function fileLeaf(f: AttachedFile, label: string): DrillLeaf<ChatLeafData> {
		return {
			type: 'leaf',
			key: `file:${f.name}`,
			label,
			// Match on the full path so a search like `sub/a` finds a nested file.
			searchableText: f.relPath ?? f.name,
			data: { fileName: f.name }
		}
	}

	// Build a nested directory tree for one linked folder. `relPath` is
	// `folder/sub/leaf` (its first segment is the folder name — see
	// `enumerateDir`), so we drill the segments below that root.
	function buildFolderBranch(folder: AttachedFolder): DrillBranch<ChatLeafData> | null {
		type Dir = { dirs: Map<string, Dir>; leaves: DrillLeaf<ChatLeafData>[] }
		const root: Dir = { dirs: new Map(), leaves: [] }
		for (const f of folder.files) {
			const segs = (f.relPath ?? f.name).split('/').slice(1) // drop the folder-name root
			let cur = root
			for (const seg of segs.slice(0, -1)) {
				let next = cur.dirs.get(seg)
				if (!next) {
					next = { dirs: new Map(), leaves: [] }
					cur.dirs.set(seg, next)
				}
				cur = next
			}
			cur.leaves.push(fileLeaf(f, segs[segs.length - 1] ?? f.name))
		}
		const byLabel = (a: { label: string }, b: { label: string }) => a.label.localeCompare(b.label)
		function toNodes(dir: Dir, keyPrefix: string): DrillNode<ChatLeafData>[] {
			const branches = [...dir.dirs.entries()]
				.map(
					([seg, sub]): DrillBranch<ChatLeafData> => ({
						type: 'branch',
						key: `${keyPrefix}/${seg}`,
						label: seg,
						icon: Folder,
						children: toNodes(sub, `${keyPrefix}/${seg}`)
					})
				)
				.sort(byLabel)
			return [...branches, ...dir.leaves.sort(byLabel)]
		}
		const children = toNodes(root, `files:${folder.name}`)
		if (children.length === 0) return null
		return {
			type: 'branch',
			key: `files:${folder.name}`,
			label: folder.name,
			icon: Folder,
			searchGroup: true,
			children
		}
	}

	function buildFilesBranch(): DrillBranch<ChatLeafData> | null {
		if (!hasAttachments) return null
		const children: DrillNode<ChatLeafData>[] = []
		for (const folder of attachedFolders) {
			const branch = buildFolderBranch(folder)
			if (branch) children.push(branch)
		}
		for (const f of attachedStandalone) children.push(fileLeaf(f, f.name))
		if (children.length === 0) return null
		return {
			type: 'branch',
			key: 'files',
			label: 'Files',
			icon: FileText,
			searchGroup: true,
			children
		}
	}

	// True when the chat root collapses to the workspace subtree (no Diffs /
	// Modules / Databases branches present). Drives handleScopeChange's
	// at-root preload — only fires when scope `[]` literally IS the workspace
	// root; otherwise we wait until the user enters the Workspace branch.
	const isWorkspaceOnly = $derived(
		!hasAttachments &&
			!availableContext.some(
				(c) =>
					(c.type === 'diff' || c.type === 'flow_module' || c.type === 'db') &&
					(!hideSelected || !isSelected(c))
			)
	)

	const tree = $derived<DrillNode<ChatLeafData>[]>(
		(() => {
			const branches: DrillNode<ChatLeafData>[] = []
			const files = buildFilesBranch()
			const diffs = buildContextBranch('diffs', 'Diffs', Diff, 'diff')
			const modules = buildContextBranch('modules', 'Modules', BarsStaggered, 'flow_module')
			const dbs = buildContextBranch('databases', 'Databases', Database, 'db')
			if (files) branches.push(files)
			if (diffs) branches.push(diffs)
			if (modules) branches.push(modules)
			if (dbs) branches.push(dbs)
			const wsChildren = buildWorkspaceTree({
				loaded: loadedForTree,
				kinds: WORKSPACE_KINDS,
				loadingKind: loader.loadingKind,
				layout: 'flat'
			}) as DrillNode<ChatLeafData>[]
			// Workspace-only (e.g. global chat with no diffs/modules/dbs): skip
			// the redundant 'Workspace' row and surface its children at the root.
			if (branches.length === 0) return wsChildren
			branches.push({
				type: 'branch',
				key: 'workspace',
				label: 'Workspace',
				icon: Layers,
				children: wsChildren,
				loading: WORKSPACE_KINDS.some((k) => loader.loadingKind[k])
			})
			return branches
		})()
	)

	function handlePick(leaf: DrillLeaf<ChatLeafData>) {
		const d = leaf.data
		if ('fileName' in d) {
			// Attached file — insert an `@filename` mention.
			onSelectFile?.(d.fileName)
		} else if ('kind' in d) {
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
			} else if (d.kind === 'app') {
				// Only raw apps reach here (the tree is narrowed in `loadedForTree`).
				const element: WorkspaceAppElement & { deletable: boolean } = {
					type: 'workspace_app',
					path: d.path,
					title: d.path,
					summary: d.summary,
					deletable: true
				}
				onSelectWorkspaceItem(element)
			}
		} else {
			// Runtime context element — added directly.
			onSelect(d)
		}
	}

	function handleScopeChange(scope: string[]) {
		// Two possible layouts:
		//   (a) WRAPPED   — `['workspace', 'dir:all:...', ...]` — chat with Diffs /
		//       Modules / Databases branches alongside Workspace.
		//   (b) UNWRAPPED — `['dir:all:...', ...]` — chat with only the workspace
		//       branch (global chat). The redundant 'workspace' wrapper is
		//       collapsed in the tree builder.
		// Empty scope `[]` is the picker root: in (a) it's the chat root
		// (don't preload — user hasn't entered Workspace yet), in (b) it's
		// the workspace root itself.
		if (scope.length === 0) {
			if (isWorkspaceOnly) loader.ensureAll()
			return
		}
		if (scope[0] === 'files') return // synthesised from attached files — no fetch
		const inWorkspace = scope[0] === 'workspace' || isWorkspaceOnly
		if (!inWorkspace) return // diffs / modules / databases — synthesised, no fetch
		// Flat workspace layout: every level is a cross-kind merge, so any
		// workspace scope needs all kinds loaded.
		loader.ensureAll()
	}

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
	{#if 'fileName' in d}
		{@const fi = getFileIcon(d.fileName)}
		{@const Icon = fi.icon}
		<Icon size={12} class="shrink-0 {fi.className ?? 'text-tertiary'}" />
	{:else if 'kind' in d}
		<RowIcon kind={d.kind} size={12} />
	{:else if d.type === 'flow_module'}
		<FlowModuleIcon module={d as unknown as FlowModule} size={14} />
	{:else}
		{@const Icon = ContextIconMap[d.type]}
		{#if Icon}<Icon size={12} class="shrink-0" />{/if}
	{/if}
{/snippet}

{#snippet branchIcon(branch: DrillBranch<ChatLeafData>)}
	{#if branch.icon}
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
		'kind' in leaf.data
			? relativizeWorkspacePath(workspaceItemDisplayPath(leaf.data), scope)
			: undefined}
	onScopeChange={handleScopeChange}
	onFilterChange={loader.onFilterChange}
	rootLoading={isWorkspaceOnly &&
		WORKSPACE_KINDS.some((k) => !loader.loaded[k] && loader.loadingKind[k])}
/>
