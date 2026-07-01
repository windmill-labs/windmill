import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'
import { getDraftItems, type DraftItem } from '$lib/workspaceDrafts.svelte'
import { getItemValue } from '$lib/utils_workspace_deploy'
import { getDraftDiffValues } from '$lib/utils_draft_deploy'
import {
	buildDeployItems,
	defaultSelection,
	diffBaseFor,
	footerSummary,
	itemsInSegment,
	segmentCounts,
	selectableOf,
	type DeployItem,
	type DeploySegment,
	type FooterSummary,
	type SessionContext
} from './sessionDeployModel'

// Reactive wrapper over the pure sessionDeployModel: owns the two async data
// sources (per-user draft list + fork comparison), builds the unified item list,
// and holds the drawer's selection + active-segment state. The diff-value loader
// lives here too so both the tree and the diff column read one resolver (it
// replaces SessionDiffDrawer's inline loadUnifiedValues/loadDraftValues).
//
// S2: read + selection only. Deploy execution (deployPlanFor → deployItem/
// deployDraft/…) and on-behalf/behind gating land in S3/S4.

export interface SessionDeployModelArgs {
	workspaceId: string
	parentWorkspaceId?: string
	parentName?: string
	isFork: boolean
	/** Chat-modified-items mask; undefined shows every draft/diff. */
	mask?: Set<string>
}

export interface DiffValues {
	before: unknown
	after: unknown
}

export function useSessionDeployModel(getArgs: () => SessionDeployModelArgs) {
	let draftItems = $state<DraftItem[]>([])
	let comparison = $state<WorkspaceComparison | undefined>(undefined)
	let draftLoading = $state(false)
	let forkLoading = $state(false)
	let draftError = $state<string | undefined>(undefined)
	let forkError = $state<string | undefined>(undefined)

	// Selection is a plain key Set; reassigned (not mutated) so Svelte tracks it.
	let selectedKeys = $state<Set<string>>(new Set())
	let segment = $state<DeploySegment>('to_review')
	// One-shot default selection: don't re-check rows the user deselected after a
	// later refetch. Reset by load() so reopening re-defaults.
	let autoSelected = false

	const context = $derived<SessionContext>({
		isFork: getArgs().isFork,
		currentWorkspaceId: getArgs().workspaceId,
		parentWorkspaceId: getArgs().parentWorkspaceId,
		parentName: getArgs().parentName
	})

	const items = $derived(
		buildDeployItems({ draftItems, comparison, mask: getArgs().mask, context })
	)
	const counts = $derived(segmentCounts(items))
	const visibleItems = $derived(itemsInSegment(items, segment))
	const footer = $derived(footerSummary(items, selectedKeys, context))

	const loading = $derived(context.isFork ? draftLoading || forkLoading : draftLoading)
	const error = $derived(context.isFork ? (forkError ?? draftError) : draftError)
	const notice = $derived(
		context.isFork && comparison?.skipped_comparison
			? 'This fork was created before change tracking was added — diffs are not available.'
			: undefined
	)

	// Seed the default selection (all selectable) once the first item set loads.
	$effect(() => {
		if (autoSelected || items.length === 0) return
		selectedKeys = new Set(defaultSelection(items))
		autoSelected = true
	})

	async function fetchDrafts() {
		draftLoading = true
		draftError = undefined
		try {
			draftItems = await getDraftItems(getArgs().workspaceId)
		} catch (e) {
			console.error('Session deploy model: draft list failed', e)
			draftError = `Failed to load drafts: ${e}`
			draftItems = []
		} finally {
			draftLoading = false
		}
	}

	async function fetchComparison() {
		const { parentWorkspaceId, workspaceId } = getArgs()
		if (!parentWorkspaceId) return
		forkLoading = true
		forkError = undefined
		try {
			comparison = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: workspaceId
			})
		} catch (e) {
			console.error('Session deploy model: comparison failed', e)
			forkError = `Failed to load comparison: ${e}`
			comparison = undefined
		} finally {
			forkLoading = false
		}
	}

	/** (Re)fetch both sources and re-arm the default selection. Called on open. */
	function load() {
		autoSelected = false
		void fetchDrafts()
		if (getArgs().isFork) void fetchComparison()
	}

	// ── Selection ──────────────────────────────────────────────────────────
	function isSelected(key: string): boolean {
		return selectedKeys.has(key)
	}
	function setSelected(keys: string[], on: boolean) {
		const next = new Set(selectedKeys)
		for (const k of keys) {
			if (on) next.add(k)
			else next.delete(k)
		}
		selectedKeys = next
	}
	function toggle(key: string) {
		setSelected([key], !selectedKeys.has(key))
	}
	function clearSelection() {
		selectedKeys = new Set()
	}
	/** Selectable keys among a set of items (drives select-all / subtree checks). */
	function selectableKeysOf(list: DeployItem[]): string[] {
		return list.filter(selectableOf).map((i) => i.key)
	}

	// ── Diff values (one resolver for tree + column) ─────────────────────────
	async function loadDiffValues(item: DeployItem): Promise<DiffValues> {
		const base = diffBaseFor(item, context)
		if (base.kind === 'draft') {
			const { deployed, draft } = await getDraftDiffValues(
				base.draftKind,
				item.path,
				base.workspaceId,
				base.draftOnly
			)
			return { before: base.draftOnly ? undefined : deployed, after: draft }
		}
		if (base.kind === 'fork_parent') {
			const [before, after] = await Promise.all([
				base.existsInParent
					? getItemValue(base.deployKind, base.path, base.parentWorkspaceId).catch(() => undefined)
					: Promise.resolve(undefined),
				base.existsInFork
					? getItemValue(base.deployKind, base.path, base.forkWorkspaceId).catch(() => undefined)
					: Promise.resolve(undefined)
			])
			return { before, after }
		}
		return { before: undefined, after: undefined }
	}

	return {
		get items() {
			return items
		},
		get visibleItems() {
			return visibleItems
		},
		get counts() {
			return counts
		},
		get footer(): FooterSummary {
			return footer
		},
		get loading() {
			return loading
		},
		get error() {
			return error
		},
		get notice() {
			return notice
		},
		get context() {
			return context
		},
		get segment() {
			return segment
		},
		setSegment(s: DeploySegment) {
			segment = s
		},
		isSelected,
		toggle,
		setSelected,
		clearSelection,
		selectableKeysOf,
		load,
		loadDiffValues
	}
}

export type SessionDeployModel = ReturnType<typeof useSessionDeployModel>
