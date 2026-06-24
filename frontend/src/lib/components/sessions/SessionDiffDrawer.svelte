<script lang="ts">
	import WorkspaceDiffDrawer, { type DiffRow } from './WorkspaceDiffDrawer.svelte'
	import { ArrowRight, GitFork, Pencil } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { WorkspaceService, type WorkspaceComparison, type WorkspaceItemDiff } from '$lib/gen'
	import { userWorkspaces } from '$lib/stores'
	import { getItemValue } from '$lib/utils_workspace_deploy'
	import { getDraftDiffValues, type DraftKind } from '$lib/utils_draft_deploy'
	import { getDraftItems, type DraftItem } from '$lib/workspaceDrafts.svelte'
	import { editUrlFor as buildEditUrl } from './forkEditUrl'
	import { maskKey, forkDiffKindToUserDraftKind } from './modifiedItemsMask'

	// Session diff drawer. For a fork session it shows ONE unified tree of every
	// chat-touched item — drafts and items already deployed-ahead of the parent —
	// each diffed against the PARENT (the merge target). The latest fork-side value
	// is the "after" (the draft when one exists, else the deployed-fork value), and
	// rows with an unsaved draft carry a "Draft" badge ("not yet deployed, must be
	// deployed before it can merge"). A non-fork session has no parent, so it falls
	// back to the plain draft diff (deployed ↔ draft).
	let {
		workspaceId,
		parentWorkspaceId,
		keys,
		chatId
	}: {
		workspaceId: string
		parentWorkspaceId?: string
		keys?: Set<string>
		chatId?: string
	} = $props()

	const isFork = $derived(!!parentWorkspaceId)
	const ws = $derived($userWorkspaces.find((w) => w.id === workspaceId))
	const parentWs = $derived(
		parentWorkspaceId ? $userWorkspaces.find((w) => w.id === parentWorkspaceId) : undefined
	)
	const forkName = $derived(ws?.name ?? workspaceId)
	const parentName = $derived(parentWs?.name ?? parentWorkspaceId ?? 'parent')

	let inner: WorkspaceDiffDrawer | undefined = $state(undefined)

	// Which baseline the fork tree diffs each row against. The rows are the same in
	// every mode (all chat-touched items); only the before/after changes:
	//   draftParent — parent ↔ latest (draft if any, else fork): the full pending
	//                 change vs the merge target (default).
	//   draftFork   — fork ↔ draft: just the unsaved (undeployed) edits.
	//   forkParent  — parent ↔ fork-deployed: what's already deployed-ahead.
	type DiffMode = 'draftParent' | 'draftFork' | 'forkParent'
	let mode = $state<DiffMode>('draftParent')

	// Draft rows carry the user-draft itemKind; the shared row icon/edit link and
	// getItemValue speak the deploy-style kinds, so map for display and back for
	// the draft value getter.
	const DEPLOY_KIND_BY_DRAFT_KIND: Partial<Record<DraftKind, string>> = {
		trigger_schedule: 'schedule',
		trigger_http: 'http_trigger',
		trigger_websocket: 'websocket_trigger',
		trigger_kafka: 'kafka_trigger',
		trigger_nats: 'nats_trigger',
		trigger_postgres: 'postgres_trigger',
		trigger_mqtt: 'mqtt_trigger',
		trigger_sqs: 'sqs_trigger',
		trigger_gcp: 'gcp_trigger',
		trigger_azure: 'azure_trigger',
		trigger_email: 'email_trigger'
	}
	function deployKindOf(it: DraftItem): string {
		const baseKind = it.raw_app ? 'raw_app' : it.kind
		return DEPLOY_KIND_BY_DRAFT_KIND[baseKind] ?? baseKind
	}

	// ── Fetched data ─────────────────────────────────────────────────────────
	let scopedDraftItems = $state<DraftItem[]>([])
	let draftLoading = $state(false)
	let draftError: string | undefined = $state(undefined)

	let comparison: WorkspaceComparison | undefined = $state(undefined)
	let forkLoading = $state(false)
	let forkError: string | undefined = $state(undefined)

	async function fetchDrafts() {
		draftLoading = true
		draftError = undefined
		try {
			const items = await getDraftItems(workspaceId)
			scopedDraftItems = keys ? items.filter((it) => keys.has(maskKey(it.kind, it.path))) : items
		} catch (e) {
			console.error('Session diff: draft list failed', e)
			draftError = `Failed to load drafts: ${e}`
			scopedDraftItems = []
		} finally {
			draftLoading = false
		}
	}

	async function fetchComparison() {
		if (!parentWorkspaceId) return
		forkLoading = true
		forkError = undefined
		try {
			comparison = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: workspaceId
			})
		} catch (e) {
			console.error('Session diff: comparison failed', e)
			forkError = `Failed to load comparison: ${e}`
			comparison = undefined
		} finally {
			forkLoading = false
		}
	}

	function statusOf(d: WorkspaceItemDiff): DiffRow['status'] {
		if (d.exists_in_fork && !d.exists_in_source) return 'added'
		if (!d.exists_in_fork && d.exists_in_source) return 'removed'
		if (d.ahead > 0 && d.behind > 0) return 'conflict'
		return 'modified'
	}

	// ── Non-fork: plain draft diff (deployed ↔ draft) ─────────────────────────
	const draftRows = $derived<DiffRow[]>(
		scopedDraftItems.map((it) => ({
			kind: deployKindOf(it),
			path: it.path,
			displayPath: it.draft_path ?? it.path,
			summary: it.summary,
			status: it.draft_only ? 'added' : 'modified'
		}))
	)
	const draftOnlyByItemKey = $derived(
		Object.fromEntries(
			scopedDraftItems.map((it) => [`${deployKindOf(it)}/${it.path}`, it.draft_only])
		)
	)

	async function loadDraftValues(d: DiffRow): Promise<{ before: unknown; after: unknown }> {
		const draftOnly = draftOnlyByItemKey[`${d.kind}/${d.path}`] ?? false
		const kind = (draftKindByItemKey[`${d.kind}/${d.path}`] ?? d.kind) as DraftKind
		const { deployed, draft } = await getDraftDiffValues(kind, d.path, workspaceId, draftOnly)
		return { before: draftOnly ? undefined : deployed, after: draft }
	}

	// ── Fork: unified tree, everything diffed against the parent ──────────────
	type RowMeta = {
		existsInParent: boolean
		existsInFork: boolean
		hasDraft: boolean
		draftKind?: DraftKind
		draftOnly: boolean
		// The item is deployed in the fork AND differs from the parent (it's in the
		// fork comparison). Only these rows belong in the "fork vs parent" view.
		deployedAhead: boolean
	}
	const draftKindByItemKey = $derived(
		Object.fromEntries(scopedDraftItems.map((it) => [`${deployKindOf(it)}/${it.path}`, it.kind]))
	)
	// Drafts keyed by canonical mask key (UserDraftItemKind:path) so fork diffs —
	// which use a different kind taxonomy — can be matched to their draft.
	const draftByCanonical = $derived(
		new Map(scopedDraftItems.map((it) => [maskKey(it.kind, it.path), it]))
	)

	const unified = $derived.by(() => {
		const rows: DiffRow[] = []
		const meta = new Map<string, RowMeta>()
		const seen = new Set<string>()
		// 1. Items deployed in the fork that differ from the parent.
		for (const d of comparison?.diffs ?? []) {
			if (!d.has_changes) continue
			const udk = forkDiffKindToUserDraftKind(d.kind)
			if (!udk) continue
			const canonical = maskKey(udk, d.path)
			if (keys && !keys.has(canonical)) continue
			seen.add(canonical)
			const draft = draftByCanonical.get(canonical)
			rows.push({
				kind: d.kind,
				path: d.path,
				status: statusOf(d),
				hasDraft: !!draft,
				displayPath: draft?.draft_path ?? d.path,
				summary: draft?.summary
			})
			meta.set(`${d.kind}/${d.path}`, {
				existsInParent: d.exists_in_source !== false,
				existsInFork: d.exists_in_fork !== false,
				hasDraft: !!draft,
				draftKind: draft?.kind,
				draftOnly: draft?.draft_only ?? false,
				deployedAhead: true
			})
		}
		// 2. Drafts with no fork-vs-parent diff (never deployed, or deployed but
		//    identical to parent until this draft). Diffed parent → draft.
		for (const it of scopedDraftItems) {
			if (seen.has(maskKey(it.kind, it.path))) continue
			const kind = deployKindOf(it)
			rows.push({
				kind,
				path: it.path,
				status: it.draft_only ? 'added' : 'modified',
				hasDraft: true,
				displayPath: it.draft_path ?? it.path,
				summary: it.summary
			})
			meta.set(`${kind}/${it.path}`, {
				existsInParent: !it.draft_only,
				existsInFork: !it.draft_only,
				hasDraft: true,
				draftKind: it.kind,
				draftOnly: it.draft_only,
				// Not in the fork comparison → no fork-vs-parent diff (never deployed,
				// or deployed but identical to parent until this draft).
				deployedAhead: false
			})
		}
		return { rows, meta }
	})

	async function loadUnifiedValues(d: DiffRow): Promise<{ before: unknown; after: unknown }> {
		const m = unified.meta.get(`${d.kind}/${d.path}`)
		if (!m) return { before: undefined, after: undefined }
		// The three values a row can have (any may be absent → renders as add/remove).
		const parentValue = () =>
			m.existsInParent
				? getItemValue(d.kind as any, d.path, parentWorkspaceId!).catch(() => undefined)
				: Promise.resolve(undefined)
		const forkValue = () =>
			m.existsInFork
				? getItemValue(d.kind as any, d.path, workspaceId).catch(() => undefined)
				: Promise.resolve(undefined)
		const draftValue = () =>
			m.hasDraft && m.draftKind
				? getDraftDiffValues(m.draftKind, d.path, workspaceId, m.draftOnly)
						.then((r) => r.draft)
						.catch(() => undefined)
				: forkValue()
		if (mode === 'forkParent') {
			const [before, after] = await Promise.all([parentValue(), forkValue()])
			return { before, after }
		}
		if (mode === 'draftFork') {
			const [before, after] = await Promise.all([forkValue(), draftValue()])
			return { before, after }
		}
		// draftParent (default): parent ↔ latest fork-side value.
		const [before, after] = await Promise.all([parentValue(), draftValue()])
		return { before, after }
	}

	// ── Active view ───────────────────────────────────────────────────────────
	// The row SET narrows per mode so the sidebar tree matches the diffs:
	//   draftParent — every chat-touched item (the full picture).
	//   draftFork   — only rows with an unsaved draft (no draft → nothing to show).
	//   forkParent  — only rows deployed-ahead of parent (a draft-only/uncommitted
	//                 item has no fork-vs-parent diff, so it's excluded).
	function metaFor(r: DiffRow): RowMeta | undefined {
		return unified.meta.get(`${r.kind}/${r.path}`)
	}
	const forkRowsForMode = $derived.by<DiffRow[]>(() => {
		if (mode === 'draftFork') return unified.rows.filter((r) => metaFor(r)?.hasDraft)
		if (mode === 'forkParent') return unified.rows.filter((r) => metaFor(r)?.deployedAhead)
		return unified.rows
	})
	const draftParentCount = $derived(unified.rows.length)
	const draftForkCount = $derived(unified.rows.filter((r) => metaFor(r)?.hasDraft).length)
	const forkParentCount = $derived(unified.rows.filter((r) => metaFor(r)?.deployedAhead).length)
	function withCount(label: string, n: number): string {
		return n > 0 ? `${label} (${n})` : label
	}

	const activeDiffs = $derived(isFork ? forkRowsForMode : draftRows)
	const activeLoading = $derived(isFork ? draftLoading || forkLoading : draftLoading)
	const activeError = $derived(isFork ? (forkError ?? draftError) : draftError)
	// Typed helper (not an inline `comparison?.…`) to avoid a $state `never`
	// inference quirk on `comparison` inside $derived.
	function skipNotice(c: WorkspaceComparison | undefined): string | undefined {
		return c?.skipped_comparison
			? 'This fork was created before change tracking was added — diffs are not available.'
			: undefined
	}
	const activeNotice = $derived(isFork ? skipNotice(comparison) : undefined)
	// In a fork the header already shows fork → parent; the per-item Draft badges
	// carry the rest, so a title would be redundant. Non-fork keeps "Drafts".
	const activeTitle = $derived(isFork ? '' : 'Drafts')
	const reviewBase = $derived(
		`/forks/compare?workspace_id=${encodeURIComponent(workspaceId)}${
			chatId ? `&from_session=${encodeURIComponent(chatId)}` : ''
		}`
	)
	// Fork → the deploy/merge (fork) view; non-fork → the draft view.
	const activeReviewHref = $derived(isFork ? reviewBase : `${reviewBase}&mode=draft`)

	export function open() {
		mode = 'draftParent'
		void fetchDrafts()
		if (isFork) void fetchComparison()
		inner?.open()
	}
</script>

<WorkspaceDiffDrawer
	bind:this={inner}
	diffs={activeDiffs}
	loadValues={isFork ? loadUnifiedValues : loadDraftValues}
	loading={activeLoading}
	error={activeError}
	notice={activeNotice}
	emptyMessage={isFork ? 'No changes from this chat in the fork.' : 'No drafts in this workspace.'}
	title={activeTitle}
	reviewHref={activeReviewHref}
	editUrlFor={(d) => buildEditUrl(d as unknown as WorkspaceItemDiff, workspaceId)}
	resetKey={isFork ? mode : undefined}
>
	{#snippet titleExtra()}
		<div class="flex items-center gap-3 text-xs text-secondary min-w-0">
			{#if isFork}
				<!-- Baseline selector. Labels name the actual workspaces (like the compare
				     page); the row set narrows per option. -->
				<ToggleButtonGroup bind:selected={mode} noWFull>
					{#snippet children({ item })}
						<ToggleButton
							value="draftParent"
							label={withCount(`Draft vs ${parentName}`, draftParentCount)}
							tooltip="Full pending change vs the merge target (default)"
							{item}
						/>
						<ToggleButton
							value="draftFork"
							label={withCount(`Draft vs ${forkName}`, draftForkCount)}
							tooltip="Unsaved (undeployed) edits in the fork"
							{item}
						/>
						<ToggleButton
							value="forkParent"
							label={withCount(`${forkName} vs ${parentName}`, forkParentCount)}
							tooltip="Already deployed in the fork, ahead of parent"
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>
				<div class="flex items-center gap-1.5 min-w-0">
					<GitFork class="w-3.5 h-3.5 shrink-0" />
					<span class="font-medium truncate" title={ws?.name ?? workspaceId}>
						{ws?.name ?? workspaceId}
					</span>
					<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
					<span class="font-medium truncate" title={parentWs?.name ?? parentWorkspaceId}>
						{parentWs?.name ?? parentWorkspaceId}
					</span>
				</div>
			{:else}
				<Pencil class="w-3.5 h-3.5 shrink-0" />
				<span class="font-medium truncate" title={ws?.name ?? workspaceId}>
					{ws?.name ?? workspaceId}
				</span>
			{/if}
		</div>
	{/snippet}
</WorkspaceDiffDrawer>
