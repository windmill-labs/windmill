<script lang="ts">
	import WorkspaceDiffDrawer, { type DiffRow } from './WorkspaceDiffDrawer.svelte'
	import { ArrowRight, GitFork, Pencil } from 'lucide-svelte'
	import { WorkspaceService, type WorkspaceComparison, type WorkspaceItemDiff } from '$lib/gen'
	import { userWorkspaces } from '$lib/stores'
	import { getItemValue } from '$lib/utils_workspace_deploy'
	import { getDraftDiffValues, type DraftKind } from '$lib/utils_draft_deploy'
	import { getDraftItems, type DraftItem } from '$lib/workspaceDrafts.svelte'
	import { editUrlFor as buildEditUrl } from './forkEditUrl'
	import { maskKey, forkDiffKindToUserDraftKind } from './modifiedItemsMask'

	// Session diff drawer. For a fork session it shows ONE tree of every chat-touched
	// item — drafts and items already deployed-ahead of the parent — and each row
	// shows its OWN change: a draft row diffs deployed-in-fork ↔ draft (the unsaved
	// edit, badged "Draft"), a deployed-ahead row diffs parent ↔ fork. A non-fork
	// session has no parent, so it shows the plain draft diff.
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

	let inner: WorkspaceDiffDrawer | undefined = $state(undefined)

	// Draft rows carry the user-draft itemKind; the shared row icon/edit link and
	// getItemValue speak the deploy-style kinds, so map for display and back for the
	// draft value getter.
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
	const draftKindByItemKey = $derived(
		Object.fromEntries(scopedDraftItems.map((it) => [`${deployKindOf(it)}/${it.path}`, it.kind]))
	)

	async function loadDraftValues(d: DiffRow): Promise<{ before: unknown; after: unknown }> {
		const draftOnly = draftOnlyByItemKey[`${d.kind}/${d.path}`] ?? false
		const kind = (draftKindByItemKey[`${d.kind}/${d.path}`] ?? d.kind) as DraftKind
		const { deployed, draft } = await getDraftDiffValues(kind, d.path, workspaceId, draftOnly)
		return { before: draftOnly ? undefined : deployed, after: draft }
	}

	// ── Fork: drafts + deployed-ahead, each diffed its own way ────────────────
	type RowMeta = {
		existsInParent: boolean
		existsInFork: boolean
		hasDraft: boolean
		draftKind?: DraftKind
		draftOnly: boolean
	}
	// Drafts keyed by canonical mask key (UserDraftItemKind:path) so fork diffs —
	// which use a different kind taxonomy — can be matched to their draft.
	const draftByCanonical = $derived(
		new Map(scopedDraftItems.map((it) => [maskKey(it.kind, it.path), it]))
	)

	const unified = $derived.by(() => {
		const rows: DiffRow[] = []
		const meta = new Map<string, RowMeta>()
		const seen = new Set<string>()
		// 1. Items deployed in the fork that differ from the parent (fork diff).
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
				// A pending draft on a deployed item shows the draft diff (modified);
				// otherwise the fork-vs-parent status.
				status: draft ? 'modified' : statusOf(d),
				hasDraft: !!draft,
				displayPath: draft?.draft_path ?? d.path,
				summary: draft?.summary
			})
			meta.set(`${d.kind}/${d.path}`, {
				existsInParent: d.exists_in_source !== false,
				existsInFork: d.exists_in_fork !== false,
				hasDraft: !!draft,
				draftKind: draft?.kind,
				draftOnly: draft?.draft_only ?? false
			})
		}
		// 2. Drafts with no fork-vs-parent diff (never deployed, or deployed but
		//    identical to parent until this draft) → the draft diff.
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
				draftOnly: it.draft_only
			})
		}
		return { rows, meta }
	})

	async function loadUnifiedValues(d: DiffRow): Promise<{ before: unknown; after: unknown }> {
		const m = unified.meta.get(`${d.kind}/${d.path}`)
		if (!m) return { before: undefined, after: undefined }
		if (m.hasDraft && m.draftKind) {
			// Draft row → its unsaved change: deployed-in-fork ↔ draft.
			const { deployed, draft } = await getDraftDiffValues(
				m.draftKind,
				d.path,
				workspaceId,
				m.draftOnly
			)
			return { before: m.draftOnly ? undefined : deployed, after: draft }
		}
		// Deployed-ahead row → parent ↔ fork.
		const [before, after] = await Promise.all([
			m.existsInParent
				? getItemValue(d.kind as any, d.path, parentWorkspaceId!).catch(() => undefined)
				: Promise.resolve(undefined),
			m.existsInFork
				? getItemValue(d.kind as any, d.path, workspaceId).catch(() => undefined)
				: Promise.resolve(undefined)
		])
		return { before, after }
	}

	// ── Active view ───────────────────────────────────────────────────────────
	const activeDiffs = $derived(isFork ? unified.rows : draftRows)
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
	// The fork header already shows fork → parent and rows carry Draft badges, so a
	// title would be redundant; non-fork keeps "Drafts".
	const activeTitle = $derived(isFork ? '' : 'Drafts')
	const reviewBase = $derived(
		`/forks/compare?workspace_id=${encodeURIComponent(workspaceId)}${
			chatId ? `&from_session=${encodeURIComponent(chatId)}` : ''
		}`
	)
	const activeReviewHref = $derived(isFork ? reviewBase : `${reviewBase}&mode=draft`)

	export function open() {
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
>
	{#snippet titleExtra()}
		<div class="flex items-center gap-1.5 text-xs text-secondary min-w-0">
			{#if isFork}
				<GitFork class="w-3.5 h-3.5 shrink-0" />
				<span class="font-medium truncate" title={ws?.name ?? workspaceId}>
					{ws?.name ?? workspaceId}
				</span>
				<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
				<span class="font-medium truncate" title={parentWs?.name ?? parentWorkspaceId}>
					{parentWs?.name ?? parentWorkspaceId}
				</span>
			{:else}
				<Pencil class="w-3.5 h-3.5 shrink-0" />
				<span class="font-medium truncate" title={ws?.name ?? workspaceId}>
					{ws?.name ?? workspaceId}
				</span>
			{/if}
		</div>
	{/snippet}
</WorkspaceDiffDrawer>
