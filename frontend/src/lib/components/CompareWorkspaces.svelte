<script lang="ts">
	import {
		AlertTriangle,
		ArrowDownRight,
		ArrowRight,
		ArrowUpRight,
		Building,
		CircleCheck,
		CircleX,
		DiffIcon,
		FileJson,
		FlaskConical,
		GitFork,
		Loader2,
		UserPlus
	} from 'lucide-svelte'
	import type { CiTestResult } from '$lib/gen'
	import { Alert, Badge } from './common'
	import {
		AppService,
		FlowService,
		FolderService,
		ResourceService,
		ScriptService,
		UserService,
		VariableService,
		WorkspaceService,
		type WorkspaceComparison,
		type WorkspaceItemDiff
	} from '$lib/gen'
	import Button from './common/button/Button.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import WorkspaceDeployItemSummary from './WorkspaceDeployItemSummary.svelte'
	import ParentWorkspaceProtectionAlert from './ParentWorkspaceProtectionAlert.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'

	import type { Kind } from '$lib/utils_deployable'
	import {
		checkDeployPermission,
		deployItem,
		deleteItemInWorkspace,
		getItemValue,
		getOnBehalfOf,
		type DeployPermission,
		type DeployResult
	} from '$lib/utils_workspace_deploy'
	import { isTriggerOrScheduleKind } from 'windmill-utils-internal'
	import Tooltip from './Tooltip.svelte'
	import OnBehalfOfSelector, {
		needsOnBehalfOfSelection,
		type OnBehalfOfChoice,
		type OnBehalfOfDetails
	} from './OnBehalfOfSelector.svelte'
	import { sendUserToast } from '$lib/toast'
	import { deepEqual } from 'fast-equals'
	import WorkspaceDeployLayout from './WorkspaceDeployLayout.svelte'
	import DeploymentRequestPanel from './deploymentRequest/DeploymentRequestPanel.svelte'
	import { userStore } from '$lib/stores'
	import { base } from '$lib/base'
	import CompareModeToggle, { type CompareMode } from './CompareModeToggle.svelte'
	import { editUrlFor } from './sessions/forkEditUrl'
	import { diffInMask } from './sessions/modifiedItemsMask'
	import DatatableSchemaDiff from './DatatableSchemaDiff.svelte'

	interface Props {
		currentWorkspaceId: string
		parentWorkspaceId: string
		comparison: WorkspaceComparison | undefined
		/** Initial merge direction; lets the page restore the chosen direction when
		 * switching back from draft mode (deploy_to → true, update → false). */
		initialMergeIntoParent?: boolean
		/** Per-direction counts for the merged toggle badges (the page owns them). */
		deployCount?: number
		updateCount?: number
		/** Draft count for the merged toggle's badge (the page owns it). */
		draftCount?: number
		/** Keys (`kind:path`) of fork items that are deployed *and* have a pending
		 * draft (has_draft). Such rows get a "+Draft" warning badge and are left
		 * out of the default selection — deploying/updating moves the deployed
		 * version, not the draft. The page derives this from the fork drafts. */
		draftKeys?: Set<string>
		/** When set (reached via a session's Review button), preselect only the
		 * diffs this chat caused (matched via diffInMask). The deploy_to default
		 * narrows to these; the update direction (parent→fork) preselects nothing,
		 * since it's never chat-caused. All rows are still shown. */
		chatMask?: Set<string>
		/** False while the (async) chatMask is still loading. The select-all default
		 * waits for this so it doesn't race the mask. Defaults to true. */
		chatMaskReady?: boolean
		/** Selecting `draft` asks the page to swap us out for CompareDrafts;
		 * deploy_to/update are handled internally but reported so the page can
		 * remember the direction. */
		onModeSelected?: (v: CompareMode) => void
		/** Fired after a deploy/update so the page re-fetches the comparison and
		 * draft count, keeping the toggle badges in sync with the new state. */
		onChanged?: () => void
	}

	let {
		currentWorkspaceId,
		parentWorkspaceId,
		comparison,
		initialMergeIntoParent = true,
		deployCount = 0,
		updateCount = 0,
		draftCount = 0,
		draftKeys = new Set<string>(),
		chatMask,
		chatMaskReady = true,
		onModeSelected,
		onChanged
	}: Props = $props()

	// Workspace-specific ("pinned") resources/variables. They keep their own
	// value per environment and are suppressed from the diff, so the page fetches
	// them separately to keep them visible and un-pinnable.
	// `onCurrent`/`onParent` track which side carries the ws_specific marker — which, because marks
	// are existence-guarded, also tells us which side the item exists on. An item present on only one
	// side can be seeded onto the other via "Create in <other>".
	type PinnedItem = { item_kind: string; path: string; onCurrent: boolean; onParent: boolean }
	let pinnedItems = $state<PinnedItem[]>([])
	let pinBusy = $state<Record<string, boolean>>({})
	let createConfirm = $state<PinnedItem | undefined>(undefined)

	// Whether this item is already workspace-specific (marked on either side); used to avoid offering
	// to pin it again from a diff row.
	const isPinned = (kind: string, path: string) =>
		pinnedItems.some((it) => it.item_kind === kind && it.path === path)

	// Bumped on each load so a response that resolves after the compared pair
	// changed can't overwrite the newer pair's items.
	let pinnedReqSeq = 0
	async function loadPinned() {
		const seq = ++pinnedReqSeq
		try {
			const [cur, par] = await Promise.all([
				WorkspaceService.listWsSpecific({ workspace: currentWorkspaceId }),
				WorkspaceService.listWsSpecific({ workspace: parentWorkspaceId })
			])
			if (seq !== pinnedReqSeq) return
			const map = new Map<string, PinnedItem>()
			const mark = (it: { item_kind: string; path: string }, side: 'cur' | 'par') => {
				const k = `${it.item_kind}:${it.path}`
				const e = map.get(k) ?? {
					item_kind: it.item_kind,
					path: it.path,
					onCurrent: false,
					onParent: false
				}
				if (side === 'cur') e.onCurrent = true
				else e.onParent = true
				map.set(k, e)
			}
			for (const it of cur) mark(it, 'cur')
			for (const it of par) mark(it, 'par')
			pinnedItems = [...map.values()].sort((a, b) =>
				`${a.item_kind}:${a.path}`.localeCompare(`${b.item_kind}:${b.path}`)
			)
		} catch (e) {
			console.error('Failed to load workspace-specific items', e)
		}
	}

	$effect(() => {
		// Re-fetch whenever the compared pair changes.
		currentWorkspaceId
		parentWorkspaceId
		loadPinned()
	})

	// Flag both environments so each side marks the item workspace-specific, not
	// only the workspace the compare is viewed from. `value=false` makes it
	// shared again.
	async function setPinned(kind: 'resource' | 'variable', path: string, value: boolean) {
		const k = `${kind}:${path}`
		pinBusy[k] = true
		try {
			const targets = [currentWorkspaceId, parentWorkspaceId]
			const results = await Promise.allSettled(
				targets.map((ws) =>
					WorkspaceService.setWsSpecific({
						workspace: ws,
						requestBody: { item_kind: kind, path, value }
					})
				)
			)
			const verb = value ? 'workspace specific' : 'shared'
			const failed = results.filter((r) => r.status === 'rejected').length
			if (failed === targets.length) {
				sendUserToast(`Failed to update workspace-specific for ${path}`, true)
			} else if (failed > 0) {
				sendUserToast(`Made ${path} ${verb} in one environment only (no access to the other)`)
			} else {
				sendUserToast(`Made ${path} ${verb}`)
			}
			await loadPinned()
			onChanged?.()
		} finally {
			pinBusy[k] = false
		}
	}

	// Collect `$var:` variable references from a resource value (mirrors the backend `collect_var_refs`).
	function collectVarRefs(value: unknown, out: string[]) {
		if (typeof value === 'string') {
			if (value.startsWith('$var:')) out.push(value.slice('$var:'.length))
		} else if (Array.isArray(value)) {
			for (const v of value) collectVarRefs(v, out)
		} else if (value && typeof value === 'object') {
			for (const v of Object.values(value)) collectVarRefs(v, out)
		}
	}

	// Create the item on the target from the source value, strictly create-only: the backend create
	// endpoints reject an existing path (`check_path_conflict`), so unlike `deployItem` — which updates
	// on conflict — this never overwrites a target created concurrently. Returns 'created', 'conflict'
	// (already present on the target), or throws for a real error.
	async function createItemOnly(
		kind: 'resource' | 'variable',
		path: string,
		from: string,
		to: string
	): Promise<'created' | 'conflict'> {
		try {
			if (kind === 'resource') {
				const r = await ResourceService.getResource({ workspace: from, path })
				await ResourceService.createResource({
					workspace: to,
					requestBody: {
						path,
						value: r.value ?? '',
						description: r.description ?? '',
						resource_type: r.resource_type
					}
				})
			} else {
				const v = await VariableService.getVariable({ workspace: from, path, decryptSecret: true })
				await VariableService.createVariable({
					workspace: to,
					requestBody: {
						path,
						value: v.value ?? '',
						is_secret: v.is_secret ?? false,
						description: v.description ?? ''
					}
				})
			}
			return 'created'
		} catch (e) {
			// Don't parse the error message: re-check existence. If the target now exists, the create
			// lost a race (the backend create is create-only and rejected it), so report a conflict and
			// leave it untouched rather than overwriting.
			const existsNow =
				kind === 'resource'
					? await ResourceService.existsResource({ workspace: to, path })
					: await VariableService.existsVariable({ workspace: to, path })
			if (existsNow) return 'conflict'
			throw e
		}
	}

	// Seed a resource's linked `$var:` variables into the target environment, copying each one's value
	// (incl. secrets) only where the target lacks it (create-only). Mirrors the backend's
	// `mark_linked_variables_ws_specific` cascade so a seeded resource resolves its references and its
	// secrets stay per-environment. Returns the paths that failed to seed.
	async function seedMissingLinkedVars(
		resourcePath: string,
		from: string,
		to: string
	): Promise<string[]> {
		const resource = await ResourceService.getResource({ workspace: from, path: resourcePath })
		const refs: string[] = []
		collectVarRefs(resource.value, refs)
		const failed: string[] = []
		for (const varPath of Array.from(new Set(refs))) {
			if (await VariableService.existsVariable({ workspace: to, path: varPath })) continue
			try {
				// 'created' and 'conflict' (now present, created concurrently) are both the desired end
				// state; only a real error counts as a failure to seed.
				await createItemOnly('variable', varPath, from, to)
			} catch (e) {
				failed.push(varPath)
			}
		}
		return failed
	}

	// Seed a workspace-specific item onto a side that lacks it, copying its current value (incl.
	// secret values). Strictly create-only: markers can be one-sided (a pin can fail on a locked
	// side), so a missing marker doesn't prove the item is missing — re-check actual existence and,
	// if the target already has it, mark it workspace-specific there instead of overwriting its value.
	async function createOnRemote(it: PinnedItem) {
		const from = it.onCurrent ? currentWorkspaceId : parentWorkspaceId
		const to = it.onCurrent ? parentWorkspaceId : currentWorkspaceId
		const k = `${it.item_kind}:${it.path}`
		const kindCast = it.item_kind as 'resource' | 'variable'
		pinBusy[k] = true
		try {
			const existsOnTarget =
				kindCast === 'resource'
					? await ResourceService.existsResource({ workspace: to, path: it.path })
					: await VariableService.existsVariable({ workspace: to, path: it.path })
			if (existsOnTarget) {
				// Don't overwrite — just mark the existing target item workspace-specific.
				await WorkspaceService.setWsSpecific({
					workspace: to,
					requestBody: { item_kind: kindCast, path: it.path, value: true }
				})
				sendUserToast(
					`${it.path} already exists in ${to} — marked it workspace-specific instead of overwriting`
				)
				await loadPinned()
				onChanged?.()
				return
			}
			// A resource owns its `$var:` secrets, so seed any linked variables the target lacks before
			// creating the resource — otherwise it would reference variables that don't exist there. If
			// any linked variable fails to seed, abort rather than create a resource with dangling refs.
			const linkedFailed =
				kindCast === 'resource' ? await seedMissingLinkedVars(it.path, from, to) : []
			if (linkedFailed.length > 0) {
				sendUserToast(
					`Did not create ${it.path} in ${to}: failed to seed linked variable(s) ${linkedFailed.join(', ')}`,
					true
				)
				return
			}

			// Create-only: a 'conflict' means the target appeared between the existence check above and
			// the create, so it was left untouched rather than overwritten.
			const result = await createItemOnly(kindCast, it.path, from, to)
			// Marking cascades to mark a resource's now-present linked variables workspace-specific.
			await WorkspaceService.setWsSpecific({
				workspace: to,
				requestBody: { item_kind: kindCast, path: it.path, value: true }
			})
			sendUserToast(
				result === 'conflict'
					? `${it.path} already exists in ${to}, marked it workspace-specific instead of overwriting`
					: `Created ${it.path} in ${to}`
			)
			await loadPinned()
			onChanged?.()
		} catch (e) {
			sendUserToast(`Failed to create ${it.path}: ${e}`, true)
		} finally {
			pinBusy[k] = false
		}
	}

	// A fork row has a pending draft when its key is in the page-provided set.
	function hasDraft(diff: WorkspaceItemDiff): boolean {
		return draftKeys.has(getItemKey(diff))
	}

	let currentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == currentWorkspaceId))
	let parentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == parentWorkspaceId))

	let mergeIntoParent = $state(initialMergeIntoParent)
	let deploying = $state(false)
	let hasAutoSelected = $state(false)
	let canDeployToParent = $state(true)
	let canPreserveInParent = $state(false)
	let canPreserveInCurrent = $state(false)
	let canPreserveOnBehalfOf = $derived(mergeIntoParent ? canPreserveInParent : canPreserveInCurrent)

	let selectableDiffs = $derived(
		comparison?.diffs.filter((diff) => {
			if (mergeIntoParent) {
				return diff.ahead > 0
			} else {
				return diff.behind > 0
			}
		}) ?? []
	)

	let selectedItems = $state<string[]>([])

	// Selected items that carry a pending draft. They're opt-in (excluded from the
	// default selection), so a non-empty list means the user explicitly picked an
	// item whose draft won't be included — confirm before deploying.
	let selectedDraftKeys = $derived(selectedItems.filter((k) => draftKeys.has(k)))
	let draftConfirmOpen = $state(false)

	function requestDeploy() {
		if (selectedDraftKeys.length > 0) {
			draftConfirmOpen = true
		} else {
			deployChanges()
		}
	}

	// Nothing actionable in the current direction (no items ahead to deploy, or
	// none behind to update). When so we show a message instead of a table of
	// greyed, non-actionable rows.
	let nothingToAct = $derived(selectableDiffs.length === 0)
	let emptyDeployMessage = $derived(
		(comparison?.diffs.length ?? 0) === 0
			? 'No changes between this fork and its parent.'
			: mergeIntoParent
				? `Nothing to deploy — ${parentWorkspaceId} already has every change from this fork.`
				: `Nothing to update — this fork is up to date with ${parentWorkspaceId}.`
	)

	let conflictingDiffs = $derived(
		comparison?.diffs.filter((diff) => diff.ahead > 0 && diff.behind > 0) ?? []
	)

	let itemsWithBehindChanges = $derived(
		comparison?.diffs.filter((diff) => {
			const status = deploymentStatus[getItemKey(diff)]?.status
			return diff && diff.behind > 0 && diff.ahead == 0 && !(status && status == 'deployed')
		}) ?? []
	)

	let itemsWithAheadChanges = $derived(
		comparison?.diffs.filter((diff) => {
			const status = deploymentStatus[getItemKey(diff)]?.status
			return diff && diff.ahead > 0 && !(status && status == 'deployed')
		}) ?? []
	)

	let hasBehindChanges = $derived(mergeIntoParent && itemsWithBehindChanges.length > 0)
	let hasAheadChanges = $derived(mergeIntoParent && itemsWithAheadChanges.length > 0)

	// Summary cache: stores summaries from both workspaces
	type SummaryCache = Record<string, { current?: string; parent?: string; loading?: boolean }>
	let summaryCache = $state<SummaryCache>({})

	// On-behalf-of tracking for flows and scripts
	// Source workspace on_behalf_of emails (keyed by workspace/kind:path)
	let onBehalfOfInfo = $state<Record<string, string | undefined>>({})
	let onBehalfOfChoice = $state<Record<string, OnBehalfOfChoice>>({})
	let customOnBehalfOf = $state<Record<string, OnBehalfOfDetails>>({})
	let deployTargetWorkspace = $derived(mergeIntoParent ? parentWorkspaceId : currentWorkspaceId)

	function getItemKey(diff: WorkspaceItemDiff): string {
		return `${diff.kind}:${diff.path}`
	}

	async function fetchSummary(
		kind: string,
		path: string,
		workspace: string
	): Promise<string | undefined> {
		try {
			if (kind === 'script') {
				const script = await ScriptService.getScriptByPath({ workspace, path })
				return script.summary
			} else if (kind === 'flow') {
				const flow = await FlowService.getFlowByPath({ workspace, path })
				return flow.summary
			} else if (kind === 'app' || kind === 'raw_app') {
				const app = await AppService.getAppByPath({ workspace, path })
				return app.summary
			} else if (kind === 'folder') {
				const folder = await FolderService.getFolder({ workspace, name: path.replace(/^f\//, '') })
				return folder.summary
			}
		} catch (error) {
			console.error(`Failed to fetch summary for ${kind}:${path}`, error)
		}
		return undefined
	}

	async function fetchSummaries(diffs: WorkspaceItemDiff[]) {
		// Only fetch summaries for scripts, flows, and apps
		const itemsToFetch = diffs.filter((diff) =>
			['script', 'flow', 'app', 'raw_app', 'folder'].includes(diff.kind)
		)

		for (const diff of itemsToFetch) {
			const key = getItemKey(diff)

			// Skip if already cached or loading
			if (summaryCache[key]) continue

			// Mark as loading
			summaryCache[key] = { loading: true }

			// Fetch from both workspaces in parallel
			const [currentSummary, parentSummary] = await Promise.all([
				fetchSummary(diff.kind, diff.path, currentWorkspaceId),
				fetchSummary(diff.kind, diff.path, parentWorkspaceId)
			])

			summaryCache[key] = {
				current: currentSummary,
				parent: parentSummary,
				loading: false
			}
		}
	}

	async function fetchOnBehalfOfInfo(diffs: WorkspaceItemDiff[]) {
		// Runnables carry an email; triggers/schedules carry `permissioned_as`.
		// `getOnBehalfOf` handles the dispatch — we just need to skip kinds that
		// have no on_behalf_of concept (resource, variable, folder, resource_type).
		const itemsWithOnBehalfOf = diffs.filter(
			(d) =>
				['flow', 'script', 'app', 'raw_app'].includes(d.kind) || isTriggerOrScheduleKind(d.kind)
		)
		for (const diff of itemsWithOnBehalfOf) {
			for (const workspace of [currentWorkspaceId, parentWorkspaceId]) {
				const workspacedKey = getWorkspacedKey(workspace, getItemKey(diff))
				if (onBehalfOfInfo[workspacedKey] !== undefined) continue

				try {
					onBehalfOfInfo[workspacedKey] = await getOnBehalfOf(
						diff.kind as Kind,
						diff.path,
						workspace
					)
				} catch {
					onBehalfOfInfo[workspacedKey] = undefined
				}
			}
		}
	}

	// Get source workspace on_behalf_of value for an item (email for runnables, permissioned_as for triggers)
	function getSourceOnBehalfOf(itemKey: string): string | undefined {
		const sourceWorkspace = mergeIntoParent ? currentWorkspaceId : parentWorkspaceId
		return onBehalfOfInfo[getWorkspacedKey(sourceWorkspace, itemKey)]
	}

	// Get target workspace on_behalf_of value for an item (existing item in destination)
	function getTargetOnBehalfOf(itemKey: string): string | undefined {
		const targetWorkspace = mergeIntoParent ? parentWorkspaceId : currentWorkspaceId
		return onBehalfOfInfo[getWorkspacedKey(targetWorkspace, itemKey)]
	}

	// Check if an item needs on_behalf_of selection
	function itemNeedsOnBehalfOfSelection(itemKey: string, kind: string): boolean {
		return needsOnBehalfOfSelection(kind, getSourceOnBehalfOf(itemKey))
	}

	// Check if all required on_behalf_of selections are made
	let hasUnselectedOnBehalfOf = $derived(
		selectedItems.some((itemKey) => {
			const diff = selectableDiffs.find((d) => getItemKey(d) === itemKey)
			if (!diff) return false
			return (
				itemNeedsOnBehalfOfSelection(itemKey, diff.kind) && onBehalfOfChoice[itemKey] === undefined
			)
		})
	)

	/**
	 * Get the on_behalf_of value for deployment based on user's choice.
	 * Returns an email for flows/scripts/apps, or permissioned_as (u/username, g/group) for triggers/schedules.
	 */
	function getOnBehalfOfForDeploy(itemKey: string, kind: Kind): string | undefined {
		const choice = onBehalfOfChoice[itemKey]
		if (choice === 'target') return getTargetOnBehalfOf(itemKey)
		if (choice === 'custom') {
			const details = customOnBehalfOf[itemKey]
			const wantsPermissionedAs = kind === 'trigger' || isTriggerOrScheduleKind(kind)
			return wantsPermissionedAs ? details?.permissionedAs : details?.email
		}
		// 'me' or undefined = don't pass, backend will use deploying user's identity
		return undefined
	}

	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let isFlow = $state(true)

	async function showDiff(kind: Kind, path: string) {
		const workspaceTo = mergeIntoParent ? parentWorkspaceId : currentWorkspaceId
		const workspaceFrom = mergeIntoParent ? currentWorkspaceId : parentWorkspaceId
		if (diffDrawer) {
			isFlow = kind == 'flow'
			diffDrawer?.openDrawer()
			let values = await Promise.all([
				getItemValue(kind, path, workspaceTo),
				getItemValue(kind, path, workspaceFrom)
			])
			diffDrawer?.setDiff({
				mode: 'simple',
				original: values?.[0] as any,
				current: values?.[1] as any,
				title: `${workspaceFrom} <> ${workspaceTo}`
			})
		}
	}

	// All *diff* items selected. Trigger items are opt-in and don't count
	// toward "all selected" — see item merge below in deployableItems.
	let allSelected = $derived(
		selectableDiffs.length > 0 &&
			selectableDiffs.every((d) => selectedItems.includes(getItemKey(d)))
	)

	async function selectAll() {
		selectedItems = selectableDiffs
			.map((d) => getItemKey(d))
			.filter((k) => !(deploymentStatus[k]?.status == 'deployed'))
	}

	function deselectAll() {
		selectedItems = []
	}

	const deploymentStatus: Record<
		string,
		{ status: 'loading' | 'deployed' | 'failed'; error?: string }
	> = $state({})

	function getWorkspacedKey(workspace: string, key: string): string {
		return `${workspace}/${key}`
	}

	async function deploy(
		kind: Kind,
		path: string,
		workspaceToDeployTo: string,
		workspaceFrom: string,
		statusKey: string
	) {
		deploymentStatus[statusKey] = { status: 'loading' }

		// Check if the item was deleted in the source workspace.
		// If so, archive/delete it in the target workspace instead of copying.
		const diff = comparison?.diffs.find((d) => getItemKey(d) === statusKey)
		const itemDeletedInSource = diff
			? mergeIntoParent
				? diff.exists_in_fork === false
				: diff.exists_in_source === false
			: false

		let result: DeployResult
		if (itemDeletedInSource) {
			result = await deleteItemInWorkspace(kind, path, workspaceToDeployTo)
		} else {
			result = await deployItem({
				kind,
				path,
				workspaceFrom,
				workspaceTo: workspaceToDeployTo,
				onBehalfOf: getOnBehalfOfForDeploy(statusKey, kind)
			})
		}

		if (result.success) {
			deploymentStatus[statusKey] = { status: 'deployed' }
		} else {
			deploymentStatus[statusKey] = { status: 'failed', error: result.error }
			sendUserToast(`Failed to deploy ${statusKey}: ${result.error}`, 'error')
		}
	}

	let deploymentErrorMessage = $state('')
	let allowBehindChangesOverride = $state(false)

	async function isComparisonUpToDate(): Promise<boolean> {
		if (!comparison) {
			return false
		}

		try {
			const result = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: currentWorkspaceId
			})

			const nonDeployedChanges = comparison.diffs.filter(
				(e) => !(deploymentStatus[getItemKey(e)]?.status == 'deployed')
			)

			if (!deepEqual(nonDeployedChanges, result.diffs)) {
				deploymentErrorMessage = `New changes detected. Please reload the page and review the new changes before ${mergeIntoParent ? 'deploying' : 'updating'}`
				return false
			}
		} catch (e) {
			deploymentErrorMessage = `Failed to check for new changes before deployment: ${e}`
			console.error('Failed to compare workspaces:', e)
			return false
		}
		return true
	}

	async function deployChanges() {
		deploying = true
		if (!(await isComparisonUpToDate())) {
			deploying = false
			return
		}

		const parent = parentWorkspaceId
		const current = currentWorkspaceId
		const sortedItems = [...selectedItems].sort((a, b) => {
			const aIsFolder = a.startsWith('folder:')
			const bIsFolder = b.startsWith('folder:')
			if (aIsFolder && !bIsFolder) return -1
			if (!aIsFolder && bIsFolder) return 1
			return 0
		})
		const to = mergeIntoParent ? parent : current
		let anyFailed = false
		// Datatables whose migrations deployed cleanly — candidates for a run prompt.
		const deployedMigrationDatatables = new Set<string>()
		for (const itemKey of sortedItems) {
			const deployable = deployableItems.find((d) => d.key === itemKey)

			if (!deployable) {
				sendUserToast(`Undeployable item: ${itemKey}`, true)
				continue
			}

			const from = mergeIntoParent ? current : parent
			await deploy(deployable.kind, deployable.path, to, from, itemKey)
			if (deploymentStatus[itemKey]?.status === 'failed') {
				anyFailed = true
			} else if (deployable.kind === 'datatable_migration') {
				deployedMigrationDatatables.add(deployable.path.split('/')[0])
			}
		}
		deploying = false
		deselectAll()

		await maybePromptRunMigrations(deployedMigrationDatatables, to)

		// If every selected item deployed cleanly and the direction was
		// merge-into-parent, resolve any open deployment request for this fork.
		if (!anyFailed && mergeIntoParent) {
			try {
				const open = await WorkspaceService.getOpenDeploymentRequest({
					workspace: currentWorkspaceId
				})
				if (open) {
					if (comparison?.all_ahead_items_visible) {
						await WorkspaceService.closeDeploymentRequestMerged({
							workspace: currentWorkspaceId,
							id: open.id
						})
						deploymentRequestPanel?.refresh()
					} else {
						// Hidden ahead changes remain: those items are excluded from the
						// list and stay undeployed, so this deploy is only partial. Closing
						// the request as "merged" (which marks its comments obsolete and
						// notifies requester/assignees of a merge) would be a lie — leave it
						// open so someone with full access can finish it.
						sendUserToast(
							'Deployed the changes visible to you. The deployment request stays open because some ahead changes are hidden from you and were not deployed.'
						)
					}
				}
			} catch (e) {
				console.error('Failed to resolve open deployment request after merge', e)
			}
		}

		// Deployed items are now in sync and should drop off the comparison; ask
		// the page to re-fetch so the list and toggle badges reflect the new state.
		onChanged?.()
	}

	/**
	 * After a deploy, offer to run the migrations of every cloned datatable that
	 * received one. `forked_from` is set on the fork's datatable config only when
	 * the datatable was cloned into a separate database — shared-DB datatables
	 * have already had the schema change applied and must not be re-run.
	 */
	async function maybePromptRunMigrations(
		deployedMigrationDatatables: Set<string>,
		runTargetWorkspace: string
	) {
		if (deployedMigrationDatatables.size === 0) return
		try {
			const forkSettings = await WorkspaceService.getPublicSettings({
				workspace: currentWorkspaceId
			})
			const datatables = forkSettings.datatable?.datatables ?? {}
			const cloned = [...deployedMigrationDatatables].filter(
				(dt) => datatables[dt]?.forked_from != null
			)
			if (cloned.length === 0) return
			runMigrationsDatatables = cloned.sort()
			runMigrationsTargetWorkspace = runTargetWorkspace
			runMigrationsModalOpen = true
		} catch (e) {
			console.error('Failed to determine cloned datatables for migration run prompt', e)
		}
	}

	async function runDeployedMigrations() {
		runMigrationsModalOpen = false
		for (const dt of runMigrationsDatatables) {
			try {
				const res = await WorkspaceService.runDatatableMigrations({
					workspace: runMigrationsTargetWorkspace,
					datatableName: dt
				})
				sendUserToast(
					`Ran ${res.applied.length} migration${res.applied.length !== 1 ? 's' : ''} on ${dt}`
				)
			} catch (e: any) {
				sendUserToast(`Failed to run migrations on ${dt}: ${e.body ?? e.message ?? e}`, true)
			}
		}
	}

	function toggleKey(key: string) {
		if (selectedItems.includes(key)) {
			selectedItems = selectedItems.filter((i) => i !== key)
		} else {
			selectedItems.push(key)
		}
	}

	function selectDefault() {
		// Triggers and schedules are opt-in: they often share runtime state with the
		// parent (kafka group_id, postgres replication slot, schedule firing time)
		// and pushing them by default would surprise users running a routine "Deploy
		// to parent" flow. The user picks them à la carte by clicking the row.
		// Items with a pending draft are also left out by default: the deployed
		// version (not the draft) is what moves, so we make the user opt in.
		// The update direction (parent→fork) is never something the chat caused, so
		// when scoped to a chat's items (chatMask set) preselect nothing there.
		if (chatMask && !mergeIntoParent) {
			selectedItems = []
			return
		}
		const filtered = selectableDiffs.filter((d) => !isTriggerOrScheduleKind(d.kind) && !hasDraft(d))
		const conflictSafe = mergeIntoParent
			? filtered
			: filtered.filter((d) => !(d.ahead > 0 && d.behind > 0))
		// When reached from a session's Review, narrow the default to this chat's items.
		const scoped = chatMask ? conflictSafe.filter((d) => diffInMask(d, chatMask)) : conflictSafe
		selectedItems = scoped
			.map((d) => getItemKey(d))
			.filter((k) => !(deploymentStatus[k]?.status == 'deployed'))
	}

	function toggleDeploymentDirection(v: string) {
		deselectAll()
		mergeIntoParent = v == 'deploy_to'
		selectDefault()
	}

	// Merged toggle: deploy_to/update flip the direction in place; draft asks the
	// page to swap us out for CompareDrafts. Either way report it up so the page
	// remembers the chosen direction across mode switches.
	function onToggleMode(v: CompareMode) {
		onModeSelected?.(v)
		if (v === 'draft') return
		toggleDeploymentDirection(v)
	}

	// Fetch user permissions for both workspaces
	$effect(() => {
		;[currentWorkspaceId, parentWorkspaceId]
		async function fetchPermissions() {
			try {
				const parentUser = await UserService.whoami({ workspace: parentWorkspaceId })
				canPreserveInParent =
					parentUser.is_admin || parentUser.groups?.includes('wm_deployers') || false
			} catch {
				canPreserveInParent = false
			}
			try {
				const currentUser = await UserService.whoami({ workspace: currentWorkspaceId })
				canPreserveInCurrent =
					currentUser.is_admin || currentUser.groups?.includes('wm_deployers') || false
			} catch {
				canPreserveInCurrent = false
			}
		}
		fetchPermissions()
	})

	// Can the user actually deploy into the target workspace? Fills the frontend
	// gap for the `RestrictDeployToDeployers` rule (+ operator), shared with the
	// session review drawer via the same checkDeployPermission util. Cached per
	// workspace; `deployPerm` tracks whichever side the current direction targets.
	let deployPerms = $state<Record<string, DeployPermission>>({})
	const deployPermFetched = new Set<string>()
	$effect(() => {
		for (const ws of [currentWorkspaceId, parentWorkspaceId]) {
			if (!ws || deployPermFetched.has(ws)) continue
			deployPermFetched.add(ws)
			void checkDeployPermission(ws).then((p) => (deployPerms = { ...deployPerms, [ws]: p }))
		}
	})
	let deployPerm = $derived(deployPerms[deployTargetWorkspace] ?? { ok: true })

	// Fetch summaries and on_behalf_of_email when comparison data loads
	$effect(() => {
		if (comparison?.diffs) {
			fetchSummaries(comparison.diffs)
			fetchOnBehalfOfInfo(comparison.diffs)
		}
	})

	// Auto-select items on initial load
	$effect(() => {
		if (comparison?.diffs && !hasAutoSelected && chatMaskReady && selectableDiffs.length > 0) {
			selectDefault()
			hasAutoSelected = true
		}
	})

	// Reset override when selection or direction changes
	$effect(() => {
		;[selectedItems, mergeIntoParent]
		allowBehindChangesOverride = false
	})

	// Trigger and schedule rows now flow through `comparison.diffs` like every
	// other deployable kind — the backend's `compareWorkspaces` populates them
	// from `workspace_diff`, with runtime fields ignored by `compare_two_*`.
	let deployableItems = $derived.by(() => {
		return (comparison?.diffs ?? [])
			.filter((diff) => {
				const key = getItemKey(diff)
				const isSelectable = selectableDiffs.includes(diff)
				const isDeployedAndIrrelevant =
					deploymentStatus[key]?.status === 'deployed' && !isSelectable
				return !isDeployedAndIrrelevant
			})
			.map((diff) => ({
				key: getItemKey(diff),
				path: diff.path,
				kind: diff.kind as Kind,
				diff
			}))
	})

	let ciTestResults = $state<Record<string, CiTestResult[]>>({})

	async function fetchCiTests() {
		if (!comparison?.diffs || !currentWorkspaceId) return
		const items = comparison.diffs
			.filter((d) => d.kind === 'script' || d.kind === 'flow' || d.kind === 'resource')
			.map((d) => ({ path: d.path, kind: d.kind as 'script' | 'flow' | 'resource' }))
		if (items.length === 0) return
		try {
			ciTestResults = await ScriptService.getCiTestResultsBatch({
				workspace: currentWorkspaceId,
				requestBody: { items }
			})
		} catch (e) {
			console.error('Failed to fetch CI test results:', e)
		}
	}

	$effect(() => {
		if (comparison && comparison.diffs.length > 0) {
			fetchCiTests()
		}
	})

	function getCiTestStatus(diff: WorkspaceItemDiff): 'pass' | 'fail' | 'running' | null {
		const key = `${diff.kind}:${diff.path}`
		const results = ciTestResults[key]
		if (!results || results.length === 0) return null
		if (results.some((r) => r.status === 'failure' || r.status === 'canceled')) return 'fail'
		if (results.some((r) => r.status === 'running' || (r.job_id && !r.status))) return 'running'
		if (results.every((r) => r.status === 'success')) return 'pass'
		return null
	}

	// Deduplicated list of all CI tests across all items
	let allCiTests = $derived.by(() => {
		const seen = new Map<string, CiTestResult>()
		for (const results of Object.values(ciTestResults)) {
			for (const r of results) {
				const existing = seen.get(r.test_script_path)
				if (
					!existing ||
					(r.started_at && (!existing.started_at || r.started_at > existing.started_at))
				) {
					seen.set(r.test_script_path, r)
				}
			}
		}
		return [...seen.values()]
	})

	let deploymentRequestPanel: DeploymentRequestPanel | undefined = $state(undefined)
	let hasOpenDeploymentRequest = $state(false)

	// After deploying datatable migrations to a cloned (separate-DB) datatable, we
	// offer to run them in the target workspace. Shared-DB datatables are skipped:
	// the schema change is already physically applied, so re-running is redundant.
	let runMigrationsModalOpen = $state(false)
	let runMigrationsDatatables = $state<string[]>([])
	let runMigrationsTargetWorkspace = $state('')
	let runMigrationsTargetWorkspaceName = $derived(
		$userWorkspaces.find((w) => w.id == runMigrationsTargetWorkspace)?.name ??
			runMigrationsTargetWorkspace
	)

	/** Display labels for trigger/schedule kinds in the merge UI. */
	const KIND_DISPLAY_NAMES: Record<string, string> = {
		schedule: 'Schedule',
		http_trigger: 'HTTP route',
		websocket_trigger: 'Websocket trigger',
		kafka_trigger: 'Kafka trigger',
		nats_trigger: 'NATS trigger',
		postgres_trigger: 'Postgres trigger',
		mqtt_trigger: 'MQTT trigger',
		amqp_trigger: 'AMQP trigger',
		sqs_trigger: 'SQS trigger',
		gcp_trigger: 'GCP trigger',
		azure_trigger: 'Azure trigger',
		email_trigger: 'Email trigger',
		datatable_migration: 'Data table migration'
	}

	// Human label for a diff kind, lowercased for inline use in the hidden-items
	// summary ("2 scripts, 1 http route").
	function hiddenKindLabel(kind: string): string {
		const base: Record<string, string> = {
			script: 'script',
			flow: 'flow',
			app: 'app',
			raw_app: 'app',
			resource: 'resource',
			variable: 'variable',
			resource_type: 'resource type',
			folder: 'folder'
		}
		return base[kind] ?? KIND_DISPLAY_NAMES[kind]?.toLowerCase() ?? kind
	}

	function formatHiddenByKind(byKind: Record<string, number>): string {
		return Object.entries(byKind)
			.map(([kind, n]) => `${n} ${hiddenKindLabel(kind)}${n !== 1 ? 's' : ''}`)
			.join(', ')
	}
</script>

{#if $workspaceStore != currentWorkspaceId}
	<Alert title="Targetting different workspace" type="info" class="my-2">
		<span>
			You are currently seeing the deployement page of workspace <b
				>{currentWorkspaceInfo?.name} ({currentWorkspaceInfo?.id})</b
			> which is not your currently selected workspace.
		</span>
		<a href="/">Click here to go home ({$workspaceStore})</a>
	</Alert>
{/if}

{#if comparison}
	{@const selectedConflicts = conflictingDiffs.filter((e) =>
		selectedItems.includes(getItemKey(e))
	).length}
	<div class="flex flex-col gap-4">
		<div class="bg-surface-tertiary p-4 rounded-md border">
			<WorkspaceDeployLayout
				items={nothingToAct ? [] : deployableItems}
				{selectedItems}
				{deploymentStatus}
				selectablePredicate={(item) => selectableDiffs.some((d) => getItemKey(d) === item.key)}
				{allSelected}
				onToggleItem={(item) => toggleKey(item.key)}
				onSelectAll={selectAll}
				onDeselectAll={deselectAll}
				emptyMessage={emptyDeployMessage}
			>
				{#snippet header()}
					<div class="flex items-center justify-between bg-surface-tertiary">
						<div class="flex flex-col gap-2 w-full pb-4">
							<div class="flex flex-wrap gap-1 items-center">
								<CompareModeToggle
									selected={mergeIntoParent ? 'deploy_to' : 'update'}
									isFork={true}
									{parentWorkspaceId}
									{deployCount}
									{updateCount}
									{draftCount}
									disabled={deploying}
									onSelected={onToggleMode}
								/>
								{#if currentWorkspaceInfo && parentWorkspaceInfo}
									<div class="flex-1 flex gap-1 items-center">
										<Badge
											color="transparent"
											class="ml-5 font-semibold"
											title={mergeIntoParent ? currentWorkspaceInfo.name : parentWorkspaceInfo.name}
										>
											<span class="text-secondary">merge:</span>
											{#if mergeIntoParent}
												<GitFork size={14} />
												<span class="text-emphasis">{currentWorkspaceInfo.id}</span>
											{:else}
												<Building size={14} />
												<span class="text-emphasis">{parentWorkspaceInfo.id}</span>
											{/if}
										</Badge>
										<ArrowRight size={16} />
										<Badge
											color="transparent"
											class="font-semibold"
											title={!mergeIntoParent
												? currentWorkspaceInfo.name
												: parentWorkspaceInfo.name}
										>
											<span class="text-secondary">into:</span>
											{#if !mergeIntoParent}
												<GitFork size={14} />
												<span class="text-emphasis">{currentWorkspaceInfo.id}</span>
											{:else}
												<Building size={14} />
												<span class="text-emphasis">{parentWorkspaceInfo.id}</span>
											{/if}
										</Badge>
									</div>
								{/if}
								<div class="flex items-center gap-2 text-sm">
									<Badge color="transparent">
										{comparison.summary.total_diffs} total items
									</Badge>
									<Badge color="transparent">
										{selectableDiffs.length}
										{mergeIntoParent ? 'deployable' : 'updateable'}
									</Badge>
									{#if conflictingDiffs.length > 0}
										<Badge color="orange">
											<AlertTriangle class="w-3 h-3 inline mr-1" />
											{conflictingDiffs.length} conflicts
										</Badge>
									{/if}
								</div>
							</div>
						</div>
					</div>
				{/snippet}
				{#snippet groupActions(groupItems)}
					<!-- Conflicts stay visible at the folder level so a group can be
					     (de)selected without scanning every row for orange badges. -->
					{@const groupConflicts = groupItems.filter((i) => {
						const d = i.diff as WorkspaceItemDiff
						return d.ahead > 0 && d.behind > 0
					}).length}
					{#if groupConflicts > 0}
						<Badge color="orange" size="xs">
							<AlertTriangle class="w-3 h-3 inline mr-0.5" />
							{groupConflicts} conflict{groupConflicts !== 1 ? 's' : ''}
						</Badge>
					{/if}
				{/snippet}
				{#if allCiTests.length > 0}
					<div class="flex flex-col gap-1.5 mt-3 p-3 border bg-surface-secondary rounded text-xs">
						<div class="flex items-center gap-1.5 font-semibold text-secondary">
							<FlaskConical size={14} />
							CI tests
						</div>
						<div class="flex flex-col gap-1">
							{#each allCiTests as test (test.test_script_path)}
								<div class="flex items-center gap-2">
									{#if test.status === 'success'}
										<CircleCheck size={14} class="text-green-600" />
									{:else if test.status === 'failure' || test.status === 'canceled'}
										<CircleX size={14} class="text-red-600" />
									{:else if test.status === 'running'}
										<Loader2 size={14} class="text-yellow-600 animate-spin" />
									{:else}
										<div class="w-3.5 h-3.5 rounded-full border border-tertiary"></div>
									{/if}
									<span class="text-secondary">{test.test_script_path}</span>
									{#if test.job_id}
										<a
											href="{base}/run/{test.job_id}?workspace={currentWorkspaceId}"
											class="text-tertiary hover:text-secondary text-2xs"
										>
											view run
										</a>
									{/if}
								</div>
							{/each}
						</div>
					</div>
				{/if}

				{#snippet alerts()}
					{#if draftCount > 0}
						<Alert title="Undeployed drafts" type="warning" size="xs" class="my-2">
							<div class="flex items-center gap-2 flex-wrap">
								<span>
									{#if mergeIntoParent}
										This workspace has {draftCount} undeployed draft{draftCount !== 1 ? 's' : ''}.
										Only deployed versions in this fork can be sent to {parentWorkspaceId} — deploy
										{draftCount !== 1 ? 'them' : 'it'} first, otherwise those changes won't be included.
									{:else}
										This workspace has {draftCount} undeployed draft{draftCount !== 1 ? 's' : ''}.
									{/if}
								</span>
								<Button variant="subtle" unifiedSize="xs" onclick={() => onModeSelected?.('draft')}>
									See drafts
								</Button>
							</div>
						</Alert>
					{/if}
					{#if mergeIntoParent}
						<ParentWorkspaceProtectionAlert
							{parentWorkspaceId}
							onUpdateCanDeploy={(canDeploy) => {
								canDeployToParent = canDeploy
							}}
						/>
					{/if}
					{#if conflictingDiffs.length > 0}
						<Alert title="Conflicting changes detected" type="warning" class="mt-2">
							<span>
								{conflictingDiffs.length} item{conflictingDiffs.length !== 1 ? 's have' : ' has'} conflicting
								changes, it was modified on the original workspace while changes were made on this fork.
								Make sure to resolve these before merging.
							</span>
						</Alert>
					{/if}
					{#if hasBehindChanges && hasAheadChanges && !(mergeIntoParent && !canDeployToParent)}
						<Alert
							title="This fork is behind {parentWorkspaceId} and needs to be up to date before deploying"
							type="warning"
							class="my-2"
						>
							You have items behind '{parentWorkspaceId}'. You need to update and test your changes
							before being able to deploy.
							<span class="font-medium flex flex-row gap-1 text-red-500">
								<input
									type="checkbox"
									bind:checked={allowBehindChangesOverride}
									class="rounded max-w-4"
								/>
								Override: Deploy despite {itemsWithBehindChanges.length} outdated item{itemsWithBehindChanges.length !==
								1
									? 's'
									: ''}
							</span>
						</Alert>
					{/if}
					{@const hiddenDir = mergeIntoParent ? comparison.hidden_ahead : comparison.hidden_behind}
					{#if hiddenDir.items.length > 0}
						<!-- Caller is an admin of this side, so the dropped items are provably
						     stale/phantom diff rows (they can see every real item). List them with
						     paths — useful for debugging the "why is this fork ahead" question. -->
						<Alert title="Hidden diff rows (visible to you as admin)" type="info" class="my-2">
							{hiddenDir.items.length}
							{mergeIntoParent ? 'ahead' : 'behind'} item{hiddenDir.items.length !== 1 ? 's' : ''}
							{hiddenDir.items.length !== 1 ? 'are' : 'is'} excluded from the list below — they are not
							resolvable as live items and are most likely stale/phantom diff rows:
							<ul class="mt-1 font-mono text-2xs">
								{#each hiddenDir.items as it}
									<li>{hiddenKindLabel(it.kind)} · {it.path}</li>
								{/each}
							</ul>
						</Alert>
					{:else if mergeIntoParent ? !comparison.all_ahead_items_visible : !comparison.all_behind_items_visible}
						<Alert title="Some changes are hidden from you" type="warning" class="my-2">
							{hiddenDir.total}
							{mergeIntoParent ? 'ahead' : 'behind'} item{hiddenDir.total !== 1 ? 's' : ''}
							({formatHiddenByKind(hiddenDir.by_kind)})
							{hiddenDir.total !== 1 ? 'are' : 'is'} not visible to your user and
							{hiddenDir.total !== 1 ? 'are' : 'is'} excluded from the list below. You can still
							{mergeIntoParent ? 'deploy' : 'update'} the items you can see — share this page with someone
							who has full access to include the rest.
						</Alert>
					{/if}
				{/snippet}

				{#snippet itemSummary(item)}
					{@const diff = item.diff as WorkspaceItemDiff}
					{@const key = item.key}
					<!-- Point the edit link at the workspace the item actually lives in:
					     a parent-only row (deleted/absent in the fork) would 404 if linked
					     into the fork, so link it into the parent instead. -->
					{@const editUrl = editUrlFor(
						diff,
						diff.exists_in_fork ? currentWorkspaceId : parentWorkspaceId
					)}
					{#if isTriggerOrScheduleKind(diff.kind)}
						<span class="text-emphasis">
							{KIND_DISPLAY_NAMES[diff.kind as string] ?? diff.kind}
						</span>
						<span class="text-tertiary mx-1">&rarr;</span>
						<span class="text-secondary">{diff.path}</span>
					{:else}
						{@const isSelectable = selectableDiffs.includes(diff)}
						{@const oldSummary = mergeIntoParent
							? summaryCache[key]?.parent
							: summaryCache[key]?.current}
						{@const newSummary = mergeIntoParent
							? summaryCache[key]?.current
							: summaryCache[key]?.parent}
						{@const existsInBothWorkspaces = !(
							(diff.exists_in_fork && !diff.exists_in_source) ||
							(!diff.exists_in_fork && diff.exists_in_source)
						)}
						<WorkspaceDeployItemSummary
							path={diff.path}
							{editUrl}
							{oldSummary}
							{newSummary}
							renamed={oldSummary != newSummary && isSelectable && existsInBothWorkspaces}
						/>
					{/if}
				{/snippet}

				{#snippet itemActions(item)}
					{@const diff = item.diff as WorkspaceItemDiff}
					{@const key = item.key}
					{@const targetOnBehalfOf = getTargetOnBehalfOf(key)}
					{@const isConflict = diff.ahead > 0 && diff.behind > 0}
					{@const existsInBothWorkspaces = !(
						(diff.exists_in_fork && !diff.exists_in_source) ||
						(!diff.exists_in_fork && diff.exists_in_source)
					)}
					<!-- On-behalf-of selector -->
					{#if itemNeedsOnBehalfOfSelection(key, diff.kind)}
						<OnBehalfOfSelector
							targetWorkspace={deployTargetWorkspace}
							targetValue={targetOnBehalfOf}
							selected={onBehalfOfChoice[key]}
							onSelect={(choice, details) => {
								onBehalfOfChoice[key] = choice
								if (details) customOnBehalfOf[key] = details
							}}
							kind={diff.kind}
							canPreserve={canPreserveOnBehalfOf}
							customValue={customOnBehalfOf[key]?.permissionedAs}
						/>
					{/if}
					{#if diff.kind === 'raw_app'}
						<Badge small icon={{ icon: FileJson }}>Raw</Badge>
					{/if}
					{#if hasDraft(diff)}
						<!-- This deployed fork item also has a pending draft. Deploying/updating
						     moves the deployed version, not the draft — so we warn (yellow, ahead
						     of the New/status badges) and leave it out of the default selection
						     (see selectDefault). -->
						<Badge
							title={mergeIntoParent
								? 'This item has a draft — deploying sends the deployed version, not the draft.'
								: 'This item has a draft — updating replaces the deployed version your draft is based on.'}
							color="yellow"
							size="xs"
						>
							<AlertTriangle class="w-3 h-3 inline mr-0.5" />+Draft
						</Badge>
					{/if}
					<!-- Status badges -->
					{#if !diff.exists_in_fork && diff.exists_in_source && diff.ahead == 0 && diff.behind > 0}
						<Badge
							title="This item was newly created in the parent workspace '{parentWorkspaceId}'"
							color="indigo"
							size="xs">New</Badge
						>
					{/if}
					{#if !diff.exists_in_fork && diff.exists_in_source && diff.ahead > 0}
						<Badge title="This item was deleted in '{currentWorkspaceId}'" color="red" size="xs"
							>Deleted</Badge
						>
					{/if}
					{#if diff.exists_in_fork && !diff.exists_in_source && diff.behind > 0}
						<Badge
							title="This item was deleted in the parent workspace '{parentWorkspaceId}'"
							color="red"
							size="xs">Deleted</Badge
						>
					{/if}
					{#if diff.exists_in_fork && !diff.exists_in_source && diff.ahead > 0 && diff.behind == 0}
						<Badge
							title="This item was newly created in '{currentWorkspaceId}'"
							color="indigo"
							size="xs">New</Badge
						>
					{/if}
					{@const ciStatus = getCiTestStatus(diff)}
					{#if ciStatus === 'pass'}
						<Badge color="green" size="xs"><CircleCheck size={10} class="mr-0.5" />CI pass</Badge>
					{:else if ciStatus === 'fail'}
						<Badge color="red" size="xs"><CircleX size={10} class="mr-0.5" />CI fail</Badge>
					{:else if ciStatus === 'running'}
						<Badge color="yellow" size="xs"
							><Loader2 size={10} class="mr-0.5 animate-spin" />CI</Badge
						>
					{/if}
					{#if !deploymentStatus[key] || deploymentStatus[key].status != 'deployed'}
						<div class="flex items-center gap-2">
							{#if isConflict || existsInBothWorkspaces}
								{#if diff.ahead > 0}
									<Badge color="green" size="xs">
										<ArrowUpRight class="w-3 h-3 inline" />
										{diff.ahead} ahead
									</Badge>
								{/if}
								{#if diff.behind > 0}
									<Badge color="blue" size="xs">
										<ArrowDownRight class="w-3 h-3 inline" />
										{diff.behind} behind
									</Badge>
								{/if}
								{#if isConflict}
									<Badge color="orange" size="xs">
										<AlertTriangle class="w-3 h-3 inline" />
										Conflict
									</Badge>
								{/if}
							{/if}
						</div>
						<div class:invisible={!existsInBothWorkspaces}>
							<Button
								unifiedSize="xs"
								variant="subtle"
								startIcon={{ icon: DiffIcon }}
								onClick={() => showDiff(diff.kind as Kind, diff.path)}
							>
								Show diff
							</Button>
						</div>
						{#if diff.kind === 'resource' || diff.kind === 'variable'}
							{#if isPinned(diff.kind, diff.path)}
								<Badge
									color="gray"
									size="xs"
									title="Kept per environment, so it's excluded from the diff. Seed a missing copy or revert it in the Workspace-specific items list below."
								>
									workspace-specific
								</Badge>
							{:else}
								<Button
									unifiedSize="xs"
									variant="subtle"
									disabled={pinBusy[key]}
									title="Keep this item's value per environment (excluded from the diff). Seed a missing copy from the Workspace-specific items list."
									onClick={() => setPinned(diff.kind as 'resource' | 'variable', diff.path, true)}
								>
									Make workspace specific
								</Button>
							{/if}
						{/if}
					{/if}
				{/snippet}

				{#snippet footer()}
					{#if !nothingToAct}
						<div class="flex items-center justify-between">
							<div></div>

							<div class="flex flex-col items-end gap-2">
								<!-- Always show the deploy footer: only the items visible to the user are
								     listed and selectable, so a partial-visibility user deploys the subset they
								     can see. The hidden items are surfaced by the non-blocking banner above,
								     not by removing the action. -->
								<div class="flex items-center gap-2">
									{#if mergeIntoParent && !hasOpenDeploymentRequest && !deploymentRequestPanel?.isDialogOpen()}
										<Button
											variant="default"
											startIcon={{ icon: UserPlus }}
											on:click={() => deploymentRequestPanel?.openRequestDialog()}
										>
											Request deployment
										</Button>
									{/if}
									<Button
										variant="accent"
										disabled={selectedItems.length === 0 ||
											deploying ||
											(hasBehindChanges && !allowBehindChangesOverride) ||
											(mergeIntoParent && !canDeployToParent) ||
											!deployPerm.ok ||
											hasUnselectedOnBehalfOf}
										title={!deployPerm.ok ? deployPerm.reason : undefined}
										loading={deploying}
										on:click={requestDeploy}
									>
										{mergeIntoParent ? 'Deploy' : 'Update'}
										{selectedItems.length} Item{selectedItems.length !== 1 ? 's' : ''}
										{#if selectedConflicts != 0}
											({selectedConflicts} conflicts)
										{/if}
									</Button>
								</div>
								{#if !deployPerm.ok}
									<span class="text-xs text-yellow-600">{deployPerm.reason}</span>
								{/if}
								{#if !(mergeIntoParent && !canDeployToParent) && hasUnselectedOnBehalfOf}
									<span class="text-xs text-yellow-600">
										You must set the "on behalf of" user for all items before deploying
										<Tooltip class="text-yellow-600">
											The "run on behalf of" field defines which user's permissions will be applied
											during execution. Make sure this is set to an appropriate user before
											deploying.
										</Tooltip>
									</span>
								{/if}

								{#if deploymentErrorMessage != ''}
									<Alert
										title="Cannot {mergeIntoParent ? 'deploy these changes' : 'update these items'}"
										type="error"
										class="my-2 max-w-80"
									>
										<span>
											{deploymentErrorMessage}
										</span>
									</Alert>
								{/if}
							</div>
						</div>
					{/if}
				{/snippet}
			</WorkspaceDeployLayout>

			<DeploymentRequestPanel
				bind:this={deploymentRequestPanel}
				forkWorkspaceId={currentWorkspaceId}
				{parentWorkspaceId}
				currentUsername={$userStore?.username ?? ''}
				isAdmin={$userStore?.is_admin ?? false}
				onStateChange={(open) => {
					hasOpenDeploymentRequest = open
				}}
			/>
		</div>

		<DatatableSchemaDiff {currentWorkspaceId} {parentWorkspaceId} />

		{#if pinnedItems.length > 0}
			<div class="bg-surface-tertiary p-4 rounded-md border flex flex-col gap-2">
				<div class="flex items-center gap-1.5 font-semibold text-secondary text-sm">
					Workspace-specific items
				</div>
				<p class="text-xs text-tertiary">
					These resources and variables keep their own value in each environment and are excluded
					from the diff. An item that exists on only one side can be seeded onto the other with
					"Create in …" (copies the current value, including secrets); it will never overwrite an
					existing value.
				</p>
				<div class="flex flex-col gap-1">
					{#each pinnedItems as it (`${it.item_kind}:${it.path}`)}
						{@const missingSide =
							it.onCurrent && !it.onParent
								? parentWorkspaceId
								: !it.onCurrent && it.onParent
									? currentWorkspaceId
									: undefined}
						<div class="flex items-center gap-2 text-sm">
							<Badge color="transparent" small>{it.item_kind}</Badge>
							<span class="text-secondary font-mono text-xs flex-1 truncate">{it.path}</span>
							{#if missingSide}
								<Button
									unifiedSize="xs"
									variant="subtle"
									disabled={pinBusy[`${it.item_kind}:${it.path}`]}
									title="Copy this item's current value (including secrets) into {missingSide}"
									onClick={() => (createConfirm = it)}
								>
									Create in {missingSide}
								</Button>
							{/if}
							<Button
								unifiedSize="xs"
								variant="subtle"
								disabled={pinBusy[`${it.item_kind}:${it.path}`]}
								onClick={() => setPinned(it.item_kind as 'resource' | 'variable', it.path, false)}
							>
								Make shared
							</Button>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>

	<DiffDrawer bind:this={diffDrawer} {isFlow} />

	<ConfirmationModal
		open={draftConfirmOpen}
		title={mergeIntoParent ? 'Deploy items with a draft?' : 'Update items with a draft?'}
		confirmationText={mergeIntoParent ? 'Deploy anyway' : 'Update anyway'}
		onConfirmed={() => {
			draftConfirmOpen = false
			deployChanges()
		}}
		onCanceled={() => (draftConfirmOpen = false)}
	>
		<div class="flex flex-col gap-2">
			<p>
				{selectedDraftKeys.length} selected item{selectedDraftKeys.length !== 1 ? 's' : ''}
				{selectedDraftKeys.length !== 1 ? 'have' : 'has'} an undeployed draft.
				{#if mergeIntoParent}
					Deploying sends the deployed version, not the draft — those draft changes won't be
					included.
				{:else}
					Updating replaces the deployed version your draft is based on.
				{/if}
			</p>
			<ul class="list-disc pl-5 text-sm font-mono text-secondary">
				{#each selectedDraftKeys as k (k)}
					<li>{k.split(':').slice(1).join(':')}</li>
				{/each}
			</ul>
		</div>
	</ConfirmationModal>

	<ConfirmationModal
		open={runMigrationsModalOpen}
		title="Run datatable migrations?"
		confirmationText="Run migrations"
		onConfirmed={runDeployedMigrations}
		onCanceled={() => (runMigrationsModalOpen = false)}
	>
		<div class="flex flex-col gap-2">
			<p>
				Run the deployed migrations in <b>{runMigrationsTargetWorkspaceName}</b> now? These data tables
				use a separate database, so the schema changes won't apply until the migrations are run.
			</p>
			<ul class="list-disc pl-5 text-sm font-mono text-secondary">
				{#each runMigrationsDatatables as dt (dt)}
					<li>{dt}</li>
				{/each}
			</ul>
		</div>
	</ConfirmationModal>

	<ConfirmationModal
		open={!!createConfirm}
		title="Create in {createConfirm?.onCurrent ? parentWorkspaceId : currentWorkspaceId}?"
		confirmationText="Create"
		onConfirmed={() => {
			const it = createConfirm
			createConfirm = undefined
			if (it) createOnRemote(it)
		}}
		onCanceled={() => (createConfirm = undefined)}
	>
		<p class="text-sm">
			This copies the current value of <span class="font-mono">{createConfirm?.path}</span>
			(including any secret value) from
			<b>{createConfirm?.onCurrent ? currentWorkspaceId : parentWorkspaceId}</b>
			into <b>{createConfirm?.onCurrent ? parentWorkspaceId : currentWorkspaceId}</b>. It stays
			workspace-specific afterward, so later promotes won't overwrite it. If it already exists
			there, it's left untouched and just marked workspace-specific.
		</p>
	</ConfirmationModal>
{:else}
	<div class="flex items-center justify-center h-full">
		<div class="text-gray-500">No comparison data available</div>
	</div>
{/if}
