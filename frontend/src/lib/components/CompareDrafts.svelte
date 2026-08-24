<script lang="ts">
	import WorkspaceDeployLayout from './WorkspaceDeployLayout.svelte'
	import { maskHasDraftRow } from './sessions/modifiedItemsMask'
	import { pipelineFolderFromBundlePath } from '$lib/pipelinePaths'
	import DiffDrawer from './DiffDrawer.svelte'
	import WorkspaceDeployItemSummary from './WorkspaceDeployItemSummary.svelte'
	import DraftBadge from './DraftBadge.svelte'
	import Toggle from './Toggle.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import { Badge } from './common'
	import Button from './common/button/Button.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { AlertTriangle, ArrowRight, DiffIcon, GitFork, Pencil, Undo2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import CompareModeToggle, { type CompareMode } from './CompareModeToggle.svelte'
	import { editUrlFor } from './sessions/forkEditUrl'
	import { AppService, FlowService, ScriptService, type WorkspaceItemDiff } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		getDraftDiffValues,
		deployDraft,
		discardDraft,
		draftBaseIsStale
	} from '$lib/utils_draft_deploy'
	import {
		checkDeployPermission,
		deployPermissionForKinds,
		type DeployPermission
	} from '$lib/utils_workspace_deploy'
	import {
		type DraftItem,
		invalidateWorkspaceDrafts,
		useWorkspaceDrafts
	} from '$lib/workspaceDrafts.svelte'
	import type { Kind as LayoutKind } from '$lib/utils_deployable'
	import { userStore, userWorkspaces } from '$lib/stores'
	import { childWorkspaceNoun } from '$lib/utils/devWorkspaceLabel'

	interface Props {
		currentWorkspaceId: string
		/** The Draft Items to review, owned by the page's Workspace Drafts resource
		 * and passed down so we don't mount a second resource (which would double
		 * the list fetches). Deploy/discard here invalidate that resource, so the
		 * list refetches upstream and the new `draftItems` flow back in. */
		draftItems: DraftItem[]
		/** True while the page's Workspace Drafts resource is loading. */
		draftsLoading?: boolean
		/** Fork context drives the parent-relative draft handling (the
		 * unchanged-from-parent filter), not the toggle. */
		isFork?: boolean
		parentWorkspaceId?: string
		/** The workspace the comparison view merges into — the parent, or an
		 * arbitrary target. Its presence is what the toggle needs: without one there
		 * is no comparison to switch back to, so the toggle is hidden. */
		compareTargetId?: string
		/** The comparison is one-way, so the toggle drops the update direction. */
		oneWayCompare?: boolean
		deployCount?: number
		updateCount?: number
		draftCount?: number
		/** When set (reached via a session's Review button), preselect only the
		 * rows this chat modified — `${UserDraftItemKind}:${path}` keys, matching
		 * Row.key. Undefined → preselect all actionable rows (the default). All
		 * rows are still shown either way. */
		chatMask?: Set<string>
		/** False while the (async) chatMask is still loading. The select-all default
		 * waits for this so it doesn't race the mask and select everything. Defaults
		 * to true for callers that don't pass a mask. */
		chatMaskReady?: boolean
		/** Selecting deploy_to/update asks the page to swap to CompareWorkspaces. */
		onModeSelected?: (v: CompareMode) => void
		/** Fired after a deploy/discard so the page can refresh the *fork*
		 * comparison (ahead/behind). The Draft Count refreshes itself — deploy/
		 * discard invalidate the Workspace Drafts resource. */
		onChanged?: () => void
	}

	let {
		currentWorkspaceId,
		draftItems,
		draftsLoading = false,
		isFork = false,
		parentWorkspaceId,
		compareTargetId,
		oneWayCompare = false,
		deployCount = 0,
		updateCount = 0,
		draftCount = 0,
		chatMask,
		chatMaskReady = true,
		onModeSelected,
		onChanged
	}: Props = $props()

	type Row = {
		/** The deploy layout's `Kind` naming (`http_trigger`, `schedule`,
		 * ...) — the layout reads `item.kind` for the row icon. Our
		 * `UserDraftItemKind` naming lives in `draftKind`. */
		kind: LayoutKind
		draftKind: DraftItem['kind']
		path: string
		/** Friendly path for display (storage `path` stays the key for all
		 * fetch/deploy/discard calls — the draft is keyed by it server-side). */
		draft_path?: string
		summary?: string
		draft_only: boolean
		legacy_draft: boolean
		raw_app: boolean
		key: string
		can_write: boolean
		draft_users?: { username?: string | null }[]
		/** The row is my own draft (or the legacy no-owner row) — only then is it
		 * actionable. Other users' rows (shown when "Show all drafts" is on) are
		 * view-only: you can't deploy/discard someone else's draft. */
		mine: boolean
		/** Fork only: true when this draft is identical to the parent's (inherited
		 * on fork, never edited here). Drives "Hide unchanged drafts". */
		unchanged_from_parent?: boolean
	}
	function getItemKey(kind: string, path: string): string {
		return `${kind}:${path}`
	}

	// UserDraftItemKind → the deploy layout's Kind union (drives row icons).
	// Trigger kinds swap the prefix to a suffix; kinds the layout doesn't
	// know (webhook, native triggers) borrow the generic 'trigger' icon.
	function toLayoutKind(kind: DraftItem['kind']): LayoutKind {
		if (kind === 'trigger_schedule') return 'schedule'
		if (kind === 'trigger_default_email') return 'email_trigger'
		if (kind.startsWith('trigger_')) {
			const candidate = `${kind.slice('trigger_'.length)}_trigger`
			const known = [
				'http_trigger',
				'websocket_trigger',
				'kafka_trigger',
				'nats_trigger',
				'postgres_trigger',
				'mqtt_trigger',
				'amqp_trigger',
				'sqs_trigger',
				'gcp_trigger',
				'azure_trigger',
				'email_trigger'
			]
			return (known.includes(candidate) ? candidate : 'trigger') as LayoutKind
		}
		return kind as LayoutKind
	}

	// "Show all drafts" widens the list from my own (+ legacy) to every user's
	// drafts in the workspace. Off by default. The default view reuses the page's
	// shared Workspace Drafts resource (passed in via `draftItems`); the "all
	// users" superset is fetched lazily here via its own resource — only while the
	// toggle is on (workspace() is undefined otherwise, so no fetch) — and shares
	// the same invalidation, so a deploy/discard refetches both.
	let showAll = $state(false)
	const allDrafts = useWorkspaceDrafts(
		() => (showAll ? currentWorkspaceId : undefined),
		() => true,
		() => (isFork ? parentWorkspaceId : undefined)
	)
	const sourceItems = $derived(showAll ? allDrafts.items : draftItems)
	const loading = $derived(showAll ? allDrafts.loading : draftsLoading)

	// On by default: a fork inherits the parent's drafts on creation, and those
	// (unrelated to the fork's own work) are the common case worth hiding.
	let hideUnchanged = $state(true)

	const currentNoun = $derived(
		childWorkspaceNoun($userWorkspaces.find((w) => w.id === currentWorkspaceId))
	)

	// The list (and, in the default view, the Draft Count) come from the Workspace
	// Drafts module; deploy/discard invalidate the resource, so the list refetches
	// and deployed items drop off without a manual reload here.
	const items: Row[] = $derived(
		sourceItems.map((d) => ({
			...d,
			key: getItemKey(d.kind, d.path),
			kind: toLayoutKind(d.kind),
			draftKind: d.kind,
			draft_path: d.draft_path,
			legacy_draft: d.legacy_draft
		}))
	)

	const currentUsername = $derived($userStore?.username)

	// Other real users (not me, not the legacy NULL-email row) who also drafted
	// this path. Only the shared full-page-editor kinds carry draft_users, so this
	// is naturally empty for drawer kinds. Deploying only deploys my own draft, so
	// a non-empty list warrants the triangle warning.
	function otherDraftUsers(row: Row): string[] {
		return (row.draft_users ?? [])
			.map((u) => u.username)
			.filter((u): u is string => !!u && u !== currentUsername)
	}

	// The backend returns exactly the rows for the current view; the only
	// client-side filter is "Hide unchanged drafts".
	const visibleItems = $derived(
		isFork && hideUnchanged ? items.filter((i) => i.unchanged_from_parent !== true) : items
	)

	// Selection gate: a row is actionable when it isn't already deployed this
	// session AND it's the user's own draft (someone else's shows view-only in the
	// "all drafts" view). A data-pipeline bundle is excluded — its scripts deploy
	// individually inside the pipeline view. Every selectable row can at least be
	// discarded: discarding your own email-scoped draft never needs write
	// permission on the path — only legacy (ownerless) drafts stay write-gated,
	// mirroring the server's discard check.
	function isDiscardable(item: Row): boolean {
		return (
			deploymentStatus[item.key]?.status !== 'deployed' &&
			item.mine &&
			item.draftKind !== 'data_pipeline' &&
			(!item.legacy_draft || item.can_write)
		)
	}

	// Deploying additionally requires write permission on the path, so the Deploy
	// count can be lower than the selection when a drafted path lost writability.
	function isDeployable(item: Row): boolean {
		return isDiscardable(item) && item.can_write
	}

	// Why a row can't be selected (drives the disabled-checkbox tooltip).
	// `undefined` ⇒ selectable.
	function blockedReason(item: Row): string | undefined {
		if (!item.mine) return 'This draft belongs to another user'
		if (item.legacy_draft && !item.can_write)
			return 'Discarding a legacy draft requires write permission on the path'
		return undefined
	}

	// Why a row can't be discarded (drives the Discard button's title).
	// Discarding only removes the caller's own draft row, so it doesn't require
	// write permission on the path — except for legacy (ownerless) drafts, which
	// the server write-gates like a deploy.
	function discardBlockedReason(item: Row): string | undefined {
		if (!item.mine) return 'This draft belongs to another user'
		if (item.legacy_draft && !item.can_write)
			return 'Discarding a legacy draft requires write permission on the path'
		return undefined
	}

	// The Draft Items list only carries the *deployed* summary, so the draft's
	// (new) display name isn't known yet. Fetch each item's draft blob once and
	// cache both names — mirrors CompareWorkspaces' fetchSummaries (eager on load,
	// keyed by row key) so the rename rendering is shared and consistent. Only
	// non-`draft_only` items can show a rename: a `draft_only` item has no deployed
	// side to diff the name against. Raw apps are fetched via the apps endpoint too
	// (it auto-detects raw from the deployed row and overlays the raw_app draft).
	const summaryCache = $state<
		Record<string, { deployed?: string; draft?: string; stale?: boolean; loading?: boolean }>
	>({})

	async function fetchDraftSummary(item: Row) {
		if (summaryCache[item.key]) return
		summaryCache[item.key] = { loading: true }
		try {
			const r = (await (item.draftKind === 'script'
				? ScriptService.getScriptByPath({
						workspace: currentWorkspaceId,
						path: item.path,
						getDraft: true
					})
				: item.draftKind === 'flow'
					? FlowService.getFlowByPath({
							workspace: currentWorkspaceId,
							path: item.path,
							getDraft: true
						})
					: AppService.getAppByPath({
							workspace: currentWorkspaceId,
							path: item.path,
							getDraft: true
						}))) as any
			const draftBlob = r.draft as any
			summaryCache[item.key] = {
				deployed: r.summary,
				draft: draftBlob?.summary,
				stale: draftBaseIsStale(item.draftKind, r),
				loading: false
			}
		} catch (error) {
			console.error(`Failed to fetch draft summary for ${item.kind}:${item.path}`, error)
			summaryCache[item.key] = { loading: false }
		}
	}

	$effect(() => {
		const current = items
		untrack(() => {
			for (const item of current) {
				if (
					item.mine &&
					!item.draft_only &&
					(['script', 'flow', 'app'].includes(item.draftKind) || item.raw_app) &&
					!summaryCache[item.key]
				) {
					void fetchDraftSummary(item)
				}
			}
		})
	})

	let selectedItems = $state<string[]>([])
	let deploying = $state(false)

	// Whether the user may deploy drafts into this workspace, via the shared util —
	// same as the fork compare page and the session review drawer. Fail-open while
	// resolving.
	let workspaceDeployPerm = $state<DeployPermission>({ ok: true })
	$effect(() => {
		const ws = currentWorkspaceId
		// Reset to fail-open on workspace change, and drop a stale resolution —
		// otherwise the previous workspace's verdict lingers (or lands last) and
		// gates the wrong workspace.
		workspaceDeployPerm = { ok: true }
		void checkDeployPermission(ws).then((p) => {
			if (ws === currentWorkspaceId) workspaceDeployPerm = p
		})
	})
	// A direct-deployment lock never reaches trigger or schedule drafts server-side, so it must
	// not disable a selection made only of those. One refused kind still blocks the whole action.
	let deployPerm = $derived(
		deployPermissionForKinds(
			workspaceDeployPerm,
			visibleItems
				.filter((i) => selectedItems.includes(i.key) && isDeployable(i))
				.map((i) => i.draftKind)
		)
	)
	// Select all on the first non-empty load (acting on everything is the common
	// intent); only once, so a refetch after a deploy doesn't re-select the
	// leftovers.
	let hasAutoSelected = $state(false)

	const deploymentStatus: Record<
		string,
		{ status: 'loading' | 'deployed' | 'failed'; error?: string }
	> = $state({})

	// Prune transient deploy status for items no longer in the list (a deployed
	// item drops off after the resource refetches). Keeps the map from growing
	// unbounded and avoids a stale 'deployed' entry suppressing a row if the same
	// kind:path is re-drafted within this mount.
	$effect(() => {
		const live = new Set(items.map((i) => i.key))
		untrack(() => {
			for (const key of Object.keys(deploymentStatus)) {
				if (!live.has(key)) delete deploymentStatus[key]
			}
		})
	})

	$effect(() => {
		if (!hasAutoSelected && chatMaskReady && visibleItems.length > 0) {
			// Default intent is act-on-all; when reached from a session's Review
			// (chatMask set), preselect only that chat's items instead.
			const selectable = visibleItems.filter(isDiscardable)
			selectedItems = (
				chatMask
					? selectable.filter((i) =>
							maskHasDraftRow(chatMask, {
								kind: i.draftKind,
								path: i.path,
								draft_path: i.draft_path
							})
						)
					: selectable
			).map((i) => i.key)
			hasAutoSelected = true
		}
	})

	// Selected items still in the visible list, per action. Derived (not a
	// pruning effect) so the footer buttons stay reactive to the Workspace
	// Drafts resource: deploy/discard drop items, and stale keys left in
	// selectedItems are simply ignored here (and by the action handlers).
	let deployableCount = $derived(
		visibleItems.filter((i) => selectedItems.includes(i.key) && isDeployable(i)).length
	)
	let discardableCount = $derived(
		visibleItems.filter((i) => selectedItems.includes(i.key) && isDiscardable(i)).length
	)
	// Selected rows the user can discard but not deploy (own draft on a path
	// without write permission) — surfaced under the footer so the diverging
	// button counts are explained.
	let undeployableSelectedCount = $derived(discardableCount - deployableCount)

	let allSelected = $derived(
		visibleItems.filter(isDiscardable).length > 0 &&
			visibleItems.filter(isDiscardable).every((i) => selectedItems.includes(i.key))
	)

	function toggleItem(item: { key: string }) {
		if (selectedItems.includes(item.key)) {
			selectedItems = selectedItems.filter((k) => k !== item.key)
		} else {
			selectedItems = [...selectedItems, item.key]
		}
	}

	function selectAll() {
		selectedItems = visibleItems.filter(isDiscardable).map((i) => i.key)
	}

	function deselectAll() {
		selectedItems = []
	}

	// --- Diff ---
	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let isFlow = $state(false)
	// Monotonic token so that two quick "Show diff" clicks don't race: a slower
	// earlier fetch must not overwrite a faster later one in the (single) drawer.
	let diffRequestId = 0

	async function showDiff(item: Row) {
		if (!diffDrawer) return
		const reqId = ++diffRequestId
		isFlow = item.draftKind === 'flow'
		diffDrawer.openDrawer()
		const { deployed, draft } = await getDraftDiffValues(
			item.draftKind,
			item.path,
			currentWorkspaceId,
			item.draft_only
		)
		// A newer Show-diff click superseded this one — drop the stale result.
		if (reqId !== diffRequestId) return
		diffDrawer.setDiff({
			mode: 'simple',
			original: deployed as any,
			current: draft as any,
			title: 'Deployed → Draft'
		})
	}

	// --- Deploy ---
	async function deploySelected() {
		deploying = true
		// Snapshot the items to deploy: deployDraft invalidates the Workspace Drafts
		// resource, so `items` can change mid-loop — iterate a stable copy. Guard on
		// isDeployable so a non-writable row can never be deployed via a stale key.
		const toDeploy = visibleItems.filter((i) => selectedItems.includes(i.key) && isDeployable(i))
		let deployedAny = false
		for (const item of toDeploy) {
			deploymentStatus[item.key] = { status: 'loading' }
			const res = await deployDraft(item.draftKind, item.path, currentWorkspaceId, {
				draftOnly: item.draft_only,
				rawApp: item.raw_app
			})
			if (res.success) {
				deploymentStatus[item.key] = { status: 'deployed' }
				deployedAny = true
			} else {
				deploymentStatus[item.key] = { status: 'failed', error: res.error }
				sendUserToast(`Failed to deploy ${item.path}: ${res.error}`, true)
			}
		}
		deploying = false
		selectedItems = []
		// The Draft list refetches itself (deployDraft invalidated it). Deploying
		// also changes the fork comparison (ahead/behind) — ask the page to refresh
		// that.
		if (deployedAny) onChanged?.()
	}

	// --- Discard ---
	// Only one discard is destructive: removing the last draft of a never-deployed
	// item (draft_only, and no other user still holds a draft) permanently deletes
	// the item, so it gets a confirmation. Every other discard just reverts to the
	// deployed version or removes your own copy while another draft remains — those
	// run immediately (the row already carries the ⚠️ for the multi-user case).
	let discardTarget = $state<Row | undefined>(undefined)

	function isDestructiveDiscard(item: Row): boolean {
		// A deployed counterpart exists → discard just reverts, never deletes.
		if (!item.draft_only) return false
		// draft_only → discarding deletes the item, UNLESS another real user still
		// holds a draft of it. Guard on `currentUsername`: if we don't yet know who
		// "me" is, `otherDraftUsers` would count my own row as someone else's, so
		// fall back to treating it as a delete (confirm) rather than risk a silent
		// deletion.
		if (!currentUsername) return true
		return otherDraftUsers(item).length === 0
	}

	function onDiscardClick(item: Row) {
		if (isDestructiveDiscard(item)) {
			discardTarget = item
		} else {
			void doDiscard(item)
		}
	}

	async function doDiscard(item: Row) {
		const res = await discardDraft(
			item.draftKind,
			item.path,
			currentWorkspaceId,
			item.draft_only,
			item.legacy_draft
		)
		if (res.success) {
			sendUserToast(item.draft_only ? `Deleted ${item.path}` : `Discarded draft of ${item.path}`)
			// discardDraft invalidated the Draft list; refresh the fork comparison.
			onChanged?.()
		} else {
			sendUserToast(`Failed to discard ${item.path}: ${res.error}`, true)
		}
	}

	function confirmDiscard() {
		const item = discardTarget
		discardTarget = undefined
		if (item) void doDiscard(item)
	}

	// --- Bulk discard ---
	// One click can drop many drafts at once, and some of them (draft_only with
	// no other drafter) are permanent deletions — always confirm, listing the
	// permanent ones explicitly.
	let bulkDiscardItems = $state<Row[] | undefined>(undefined)
	let discarding = $state(false)
	const bulkPermanent = $derived((bulkDiscardItems ?? []).filter(isDestructiveDiscard))
	// Third outcome the modal must cover: a draft-only item someone else also
	// drafted isn't deleted — only this user's draft goes; the item survives via
	// the other drafts.
	const bulkSharedCount = $derived(
		(bulkDiscardItems ?? []).filter((i) => i.draft_only && !isDestructiveDiscard(i)).length
	)

	function onDiscardSelectedClick() {
		const toDiscard = visibleItems.filter((i) => selectedItems.includes(i.key) && isDiscardable(i))
		if (toDiscard.length > 0) bulkDiscardItems = toDiscard
	}

	async function discardSelected(toDiscard: Row[]) {
		discarding = true
		let changed = false
		for (const item of toDiscard) {
			deploymentStatus[item.key] = { status: 'loading' }
			// invalidate: false — one refetch after the whole batch (below), not
			// one per row.
			const res = await discardDraft(
				item.draftKind,
				item.path,
				currentWorkspaceId,
				item.draft_only,
				item.legacy_draft,
				false
			)
			if (res.success) {
				changed = true
				delete deploymentStatus[item.key]
			} else {
				deploymentStatus[item.key] = { status: 'failed', error: res.error }
				sendUserToast(`Failed to discard ${item.path}: ${res.error}`, true)
			}
		}
		discarding = false
		selectedItems = []
		if (changed) {
			// Refetch the Draft list once for the batch, then refresh the fork
			// comparison.
			invalidateWorkspaceDrafts(currentWorkspaceId)
			onChanged?.()
		}
	}

	function confirmBulkDiscard() {
		const toDiscard = bulkDiscardItems
		bulkDiscardItems = undefined
		if (toDiscard) void discardSelected(toDiscard)
	}

	// Editor URL for a draft item, scoped to the current workspace. Raw apps live
	// under a different editor route, so map their kind accordingly. Kinds whose
	// editor is a drawer on a list page (variables, resources, schedules,
	// triggers) link to that page with the item path as the hash anchor.
	const LIST_PAGE_FOR_KIND: Partial<Record<Row['draftKind'], string>> = {
		variable: '/variables',
		resource: '/resources',
		trigger_schedule: '/schedules',
		trigger_http: '/routes',
		trigger_websocket: '/websocket_triggers',
		trigger_postgres: '/postgres_triggers',
		trigger_kafka: '/kafka_triggers',
		trigger_nats: '/nats_triggers',
		trigger_mqtt: '/mqtt_triggers',
		trigger_amqp: '/amqp_triggers',
		trigger_sqs: '/sqs_triggers',
		trigger_gcp: '/gcp_triggers',
		trigger_azure: '/azure_triggers',
		trigger_email: '/email_triggers'
	}
	function draftEditUrl(d: Row): string | undefined {
		if (d.draftKind === 'data_pipeline') {
			const folder = pipelineFolderFromBundlePath(d.path)
			return folder
				? `/pipeline/${encodeURIComponent(folder)}?workspace=${encodeURIComponent(currentWorkspaceId)}`
				: undefined
		}
		const listPage = LIST_PAGE_FOR_KIND[d.draftKind]
		if (listPage) {
			return `${listPage}?workspace=${encodeURIComponent(currentWorkspaceId)}#${d.path}`
		}
		return editUrlFor(
			{ kind: d.raw_app ? 'raw_app' : d.draftKind, path: d.path } as unknown as WorkspaceItemDiff,
			currentWorkspaceId
		)
	}

	// Auto-generated draft slot: `u/{user}/draft_{uuid}` (uuid dashes → underscores),
	// minted for a never-named draft. We don't surface this synthetic id as a row's
	// bold title.
	const AUTO_GEN_DRAFT_RE = /(^|\/)draft_[0-9a-f]{8}(_[0-9a-f]{4}){3}_[0-9a-f]{12}$/

	// Bold title for a row: the friendly typed path if the draft carries one, else
	// the storage path — with `{user}` truncated at `@` (the admins workspace and
	// email-as-username setups put the full email in the namespace). The real
	// path/key (used for fetch/deploy/discard) is left untouched. Returns '' for an
	// auto-generated `draft_{uuid}` path so it isn't shown in bold (the row still
	// shows the storage path in its secondary line).
	function displayPath(d: Row): string {
		// The pipeline bundle's storage path (`f/<folder>/data_pipeline`) is an
		// implementation detail — show the folder it belongs to.
		if (d.draftKind === 'data_pipeline') {
			const folder = pipelineFolderFromBundlePath(d.path)
			return folder ? `f/${folder}` : d.path
		}
		const path = d.draft_path ?? d.path
		if (AUTO_GEN_DRAFT_RE.test(path)) return ''
		const segs = path.split('/')
		if (segs[0] === 'u' && segs.length >= 2) {
			const at = segs[1].indexOf('@')
			if (at > 0) segs[1] = segs[1].slice(0, at)
		}
		return segs.join('/')
	}

	// Human label for the kind badge on each row — without it a variable
	// draft and a script draft at the same path are indistinguishable.
	function kindLabel(kind: Row['draftKind']): string {
		if (kind === 'raw_app') return 'app'
		if (kind === 'data_pipeline') return 'pipeline'
		if (kind === 'trigger_schedule') return 'schedule'
		if (kind.startsWith('trigger_')) return `${kind.slice('trigger_'.length)} trigger`
		return kind
	}
</script>

<div class="flex flex-col gap-4">
	<div class="bg-surface-tertiary p-4 rounded-md border">
		<WorkspaceDeployLayout
			items={visibleItems}
			{selectedItems}
			{deploymentStatus}
			{allSelected}
			selectablePredicate={(item) => isDiscardable(item as unknown as Row)}
			selectBlockedReason={(item) => blockedReason(item as unknown as Row)}
			onToggleItem={toggleItem}
			onSelectAll={selectAll}
			onDeselectAll={deselectAll}
			emptyMessage={loading
				? 'Loading drafts…'
				: isFork && hideUnchanged && items.length > 0
					? 'All drafts are unchanged from the parent workspace. Turn off "Hide unchanged drafts" to show them.'
					: showAll
						? 'No drafts in this workspace'
						: 'No drafts you authored in this workspace'}
		>
			{#snippet selectAllActions()}
				<div class="flex items-center gap-4">
					{#if isFork}
						<Toggle
							bind:checked={hideUnchanged}
							size="xs"
							options={{
								right: 'Hide unchanged drafts',
								rightTooltip: `Hide drafts identical to the parent workspace. A ${currentNoun} inherits the parent's drafts when it's created; those are unrelated to the changes made in this ${currentNoun}.`
							}}
						/>
					{/if}
					<Toggle
						bind:checked={showAll}
						size="xs"
						options={{
							right: 'Show all drafts',
							rightTooltip:
								"Show every user's drafts in this workspace, not just yours. Others' drafts are view-only — you can only deploy or discard your own."
						}}
					/>
				</div>
			{/snippet}

			{#snippet header()}
				{#if compareTargetId}
					<div class="flex flex-wrap gap-1 items-center bg-surface-tertiary pb-4">
						<CompareModeToggle
							selected="draft"
							isFork={true}
							oneWay={oneWayCompare}
							parentWorkspaceId={compareTargetId}
							{deployCount}
							{updateCount}
							{draftCount}
							disabled={deploying}
							onSelected={(v) => onModeSelected?.(v)}
						/>
						<!-- Direction badge, mirroring the fork compare header: make it explicit
						     that deploying a draft promotes it *within this fork* (deployed↔draft),
						     not up to the parent. -->
						<div class="flex-1 flex gap-1 items-center">
							<Badge color="transparent" class="ml-5 font-semibold">
								<span class="text-secondary">deploy:</span>
								<Pencil size={14} />
								<span class="text-emphasis">draft</span>
							</Badge>
							<ArrowRight size={16} />
							<Badge color="transparent" class="font-semibold" title={currentWorkspaceId}>
								<span class="text-secondary">into:</span>
								<GitFork size={14} />
								<span class="text-emphasis">{currentWorkspaceId}</span>
							</Badge>
						</div>
					</div>
				{/if}
			{/snippet}

			{#snippet itemSummary(item)}
				{@const draftItem = item as unknown as Row}
				{@const editUrl = draftEditUrl(draftItem)}
				{@const cache = summaryCache[draftItem.key]}
				{@const oldSummary = cache?.deployed ?? draftItem.summary}
				{@const newSummary = cache?.draft ?? draftItem.summary}
				<WorkspaceDeployItemSummary
					path={displayPath(draftItem)}
					{editUrl}
					{oldSummary}
					{newSummary}
					renamed={!draftItem.draft_only &&
						!!oldSummary &&
						!!newSummary &&
						oldSummary !== newSummary}
				/>
			{/snippet}

			{#snippet itemPath(item)}
				{@const draftItem = item as unknown as Row}
				{#if draftItem.kind === 'resource' || draftItem.kind === 'variable' || draftItem.kind === 'resource_type'}
					<!-- drawer-only items show their path as the title; keep this line empty -->
				{:else if !draftItem.draft_only && draftItem.draft_path && draftItem.draft_path !== draftItem.path}
					<!-- Path rename: a *deployed* item's draft moves it to a new path. Strike
					     the deployed path and show the draft's target path. Draft-only items are
					     excluded — their storage path is an auto-generated `draft_{uuid}` and
					     `draft_path` is just the pretty name, not a rename. -->
					<span class="line-through">{draftItem.path}</span>
					{draftItem.draft_path}
				{:else}
					{draftItem.draft_path ?? draftItem.path}
				{/if}
			{/snippet}

			{#snippet itemActions(item)}
				{@const draftItem = item as unknown as Row}
				{@const others = otherDraftUsers(draftItem)}
				<Badge color="gray" size="xs">{kindLabel(draftItem.draftKind)}</Badge>
				<DraftBadge
					is_draft={true}
					draft_only={draftItem.draft_only}
					draft_users={draftItem.draft_users ?? []}
					{currentUsername}
					workspace={currentWorkspaceId}
					itemKind={draftItem.draftKind}
					path={draftItem.path}
					allowFork={false}
				/>
				{#if draftItem.mine && others.length > 0}
					<Popover openOnHover debounceDelay={50}>
						{#snippet trigger()}
							<AlertTriangle size={16} class="text-yellow-500" />
						{/snippet}
						{#snippet content()}
							<div class="text-xs p-3 max-w-xs text-primary">
								{others.length} other {others.length === 1 ? 'user' : 'users'} ({others.join(', ')})
								{others.length === 1 ? 'has' : 'have'} a draft of this item. Deploying only deploys your
								draft; theirs are left untouched.
							</div>
						{/snippet}
					</Popover>
				{/if}
				{#if draftItem.mine && summaryCache[draftItem.key]?.stale}
					<Popover openOnHover debounceDelay={50}>
						{#snippet trigger()}
							<AlertTriangle size={16} class="text-orange-500" />
						{/snippet}
						{#snippet content()}
							<div class="text-xs p-3 max-w-xs text-primary">
								Started from an older deployed version. A newer version was deployed after this
								draft began. Review the latest deploy before deploying.
							</div>
						{/snippet}
					</Popover>
				{/if}
				{#if deploymentStatus[draftItem.key]?.status !== 'deployed'}
					{#if draftItem.draftKind === 'data_pipeline'}
						<!-- A bundle isn't diffable/deployable here — its scripts deploy
						     individually inside the pipeline view. -->
						{@const openUrl = draftEditUrl(draftItem)}
						{#if openUrl}
							<Button
								unifiedSize="xs"
								variant="subtle"
								startIcon={{ icon: ArrowRight }}
								href={openUrl}
							>
								Open pipeline
							</Button>
						{/if}
					{:else}
						{@const discardBlock = discardBlockedReason(draftItem)}
						<!-- Show diff fetches the *current user's* draft overlay, so it's only
						     meaningful for your own/legacy rows. Another user's draft (view-only,
						     `mine=false`) would diff against the wrong draft or 404 — hide it. -->
						{#if draftItem.mine}
							<Button
								unifiedSize="xs"
								variant="subtle"
								startIcon={{ icon: DiffIcon }}
								onClick={() => showDiff(draftItem)}
							>
								Show diff
							</Button>
						{/if}
						<Button
							unifiedSize="xs"
							variant="subtle"
							destructive
							disabled={!!discardBlock}
							title={discardBlock}
							startIcon={{ icon: Undo2 }}
							onClick={() => onDiscardClick(draftItem)}
						>
							Discard draft
						</Button>
					{/if}
				{/if}
			{/snippet}

			{#snippet footer()}
				<div class="flex flex-col items-end gap-2">
					<div class="flex items-center gap-2">
						<Button
							variant="default"
							destructive
							disabled={discardableCount === 0 || deploying || discarding}
							loading={discarding}
							startIcon={{ icon: Undo2 }}
							onClick={onDiscardSelectedClick}
						>
							Discard {discardableCount} draft{discardableCount !== 1 ? 's' : ''}
						</Button>
						<Button
							variant="accent"
							disabled={deployableCount === 0 || deploying || discarding || !deployPerm.ok}
							title={!deployPerm.ok ? deployPerm.reason : undefined}
							loading={deploying}
							onClick={deploySelected}
						>
							Deploy {deployableCount} draft{deployableCount !== 1 ? 's' : ''}
						</Button>
					</div>
					{#if !deployPerm.ok}
						<span class="text-xs text-yellow-600">{deployPerm.reason}</span>
					{:else if undeployableSelectedCount > 0}
						<span class="text-xs text-secondary">
							{undeployableSelectedCount} selected draft{undeployableSelectedCount !== 1 ? 's' : ''}
							can't be deployed (no write permission on the path) but can still be discarded
						</span>
					{/if}
				</div>
			{/snippet}
		</WorkspaceDeployLayout>
	</div>

	<DiffDrawer bind:this={diffDrawer} {isFlow} />
</div>

<ConfirmationModal
	open={bulkDiscardItems !== undefined}
	title="Discard selected drafts"
	confirmationText="Discard"
	onConfirmed={confirmBulkDiscard}
	onCanceled={() => (bulkDiscardItems = undefined)}
>
	<p>
		This will discard {bulkDiscardItems?.length} draft{(bulkDiscardItems?.length ?? 0) !== 1
			? 's'
			: ''}. Items with a deployed version revert to it.
	</p>
	{#if bulkSharedCount > 0}
		<p class="mt-2">
			{bulkSharedCount} draft-only {bulkSharedCount === 1 ? 'item is' : 'items are'} also drafted by
			other users: only your draft is removed and the {bulkSharedCount === 1 ? 'item' : 'items'} will
			remain through theirs.
		</p>
	{/if}
	{#if bulkPermanent.length > 0}
		<p class="mt-2">
			{bulkPermanent.length}
			{bulkPermanent.length === 1 ? 'item exists' : 'items exist'} only as a draft and will be
			<span class="font-semibold">permanently deleted</span>:
		</p>
		<ul class="list-disc list-inside font-mono text-xs mt-1">
			{#each bulkPermanent as item (item.key)}
				<li>{item.draft_path ?? item.path}</li>
			{/each}
		</ul>
	{/if}
</ConfirmationModal>

<!-- Only the destructive discard (deleting the last draft of a never-deployed
     item) opens this modal; non-destructive discards run without confirmation. -->
<ConfirmationModal
	open={discardTarget !== undefined}
	title="Delete item"
	confirmationText="Delete"
	onConfirmed={confirmDiscard}
	onCanceled={() => (discardTarget = undefined)}
>
	<p>
		<span class="font-mono font-medium text-primary"
			>{discardTarget?.draft_path ?? discardTarget?.path}</span
		> exists only as a draft. Discarding it will permanently delete the item. This cannot be undone.
	</p>
</ConfirmationModal>
