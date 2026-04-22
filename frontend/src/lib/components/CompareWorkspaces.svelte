<script lang="ts">
	import {
		AlertTriangle,
		ArrowDown,
		ArrowDownRight,
		ArrowRight,
		ArrowUp,
		ArrowUpRight,
		Building,
		CircleCheck,
		CircleX,
		DiffIcon,
		Eye,
		FileJson,
		FlaskConical,
		GitFork,
		Loader2,
		Trash2,
		UserPlus
	} from 'lucide-svelte'
	import type { CiTestResult } from '$lib/gen'
	import { Alert, Badge } from './common'
	import {
		AppService,
		EmailTriggerService,
		FlowService,
		FolderService,
		GcpTriggerService,
		HttpTriggerService,
		KafkaTriggerService,
		MqttTriggerService,
		NatsTriggerService,
		PostgresTriggerService,
		ScheduleService,
		ScriptService,
		SqsTriggerService,
		UserService,
		WebsocketTriggerService,
		WorkspaceService,
		type WorkspaceComparison,
		type WorkspaceItemDiff
	} from '$lib/gen'
	import Button from './common/button/Button.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import ParentWorkspaceProtectionAlert from './ParentWorkspaceProtectionAlert.svelte'
	import ScheduleEditor from './triggers/schedules/ScheduleEditor.svelte'
	import RouteEditor from './triggers/http/RouteEditor.svelte'
	import WebsocketTriggerEditor from './triggers/websocket/WebsocketTriggerEditor.svelte'
	import KafkaTriggerEditor from './triggers/kafka/KafkaTriggerEditor.svelte'
	import PostgresTriggerEditor from './triggers/postgres/PostgresTriggerEditor.svelte'
	import NatsTriggerEditor from './triggers/nats/NatsTriggerEditor.svelte'
	import MqttTriggerEditor from './triggers/mqtt/MqttTriggerEditor.svelte'
	import SqsTriggerEditor from './triggers/sqs/SqsTriggerEditor.svelte'
	import GcpTriggerEditor from './triggers/gcp/GcpTriggerEditor.svelte'
	import EmailTriggerEditor from './triggers/email/EmailTriggerEditor.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'

	import type { Kind } from '$lib/utils_deployable'
	import {
		deployItem,
		deleteItemInWorkspace,
		getItemValue,
		getOnBehalfOf,
		type DeployResult
	} from '$lib/utils_workspace_deploy'
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
	import type { TriggerKind } from './triggers'
	import { triggerDisplayNamesMap, triggerKindToTriggerType } from './triggers/utils'
	import { getEmailAddress, getEmailDomain } from './triggers/email/utils'
	import { base } from '$lib/base'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import DatatableSchemaDiff from './DatatableSchemaDiff.svelte'

	interface Props {
		currentWorkspaceId: string
		parentWorkspaceId: string
		comparison: WorkspaceComparison | undefined
	}

	let { currentWorkspaceId, parentWorkspaceId, comparison }: Props = $props()

	let currentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == currentWorkspaceId))
	let parentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == parentWorkspaceId))

	let mergeIntoParent = $state(true)
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
		const flowsAndScripts = diffs.filter((d) =>
			['flow', 'script', 'app', 'raw_app'].includes(d.kind)
		)
		for (const diff of flowsAndScripts) {
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
			return kind === 'trigger' ? details?.permissionedAs : details?.email
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

	async function selectAllNonConflicts() {
		selectedItems = selectableDiffs
			.filter((d) => !(d.ahead > 0 && d.behind > 0))
			.map((d) => getItemKey(d))
			.filter((k) => !(deploymentStatus[k]?.status == 'deployed'))
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
		statusKey: string,
		trigger?: ForkTrigger
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
		} else if (trigger) {
			result = await deployItem({
				kind: 'trigger',
				path,
				workspaceFrom,
				workspaceTo: workspaceToDeployTo,
				additionalInformation: { triggers: { kind: trigger.triggerKind } },
				onBehalfOf: getOnBehalfOfForDeploy(statusKey, 'trigger')
			})
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
			sendUserToast(`Failed to deploy ${statusKey}: ${result.error}`)
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
		let anyFailed = false
		for (const itemKey of sortedItems) {
			const deployable = deployableItems.find((d) => d.key === itemKey)

			if (!deployable) {
				sendUserToast(`Undeployable item: ${itemKey}`, true)
				continue
			}

			const to = mergeIntoParent ? parent : current
			const from = mergeIntoParent ? current : parent
			await deploy(deployable.kind, deployable.path, to, from, itemKey, deployable.trigger)
			if (deploymentStatus[itemKey]?.status === 'failed') {
				anyFailed = true
			}
		}
		deploying = false
		deselectAll()

		// If every selected item deployed cleanly and the direction was
		// merge-into-parent, close any open deployment request for this fork.
		if (!anyFailed && mergeIntoParent) {
			try {
				const open = await WorkspaceService.getOpenDeploymentRequest({
					workspace: currentWorkspaceId
				})
				if (open) {
					await WorkspaceService.closeDeploymentRequestMerged({
						workspace: currentWorkspaceId,
						id: open.id
					})
					deploymentRequestPanel?.refresh()
				}
			} catch (e) {
				console.error('Failed to close open deployment request after merge', e)
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
		if (mergeIntoParent) {
			selectAll()
		} else {
			selectAllNonConflicts()
		}
	}

	function toggleDeploymentDirection(v: string) {
		deselectAll()
		mergeIntoParent = v == 'deploy_to'
		selectDefault()
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

	// Fetch summaries and on_behalf_of_email when comparison data loads
	$effect(() => {
		if (comparison?.diffs) {
			fetchSummaries(comparison.diffs)
			fetchOnBehalfOfInfo(comparison.diffs)
		}
	})

	// Auto-select items on initial load
	$effect(() => {
		if (comparison?.diffs && !hasAutoSelected && selectableDiffs.length > 0) {
			selectDefault()
			hasAutoSelected = true
		}
	})

	// Reset override when selection or direction changes
	$effect(() => {
		;[selectedItems, mergeIntoParent]
		allowBehindChangesOverride = false
	})

	function getTriggerKey(trigger: ForkTrigger): string {
		return `trigger:${trigger.triggerKind}:${trigger.path}`
	}

	// Transform diffs + fork triggers to deployable item format for the
	// shared layout. Triggers render as inline rows alongside diff items;
	// they carry no ahead/behind info and are selectable à la carte.
	let deployableItems = $derived.by(() => {
		const diffItems = (comparison?.diffs ?? [])
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
				diff,
				trigger: undefined as ForkTrigger | undefined
			}))
		const triggerItems = forkTriggers
			.filter((t) => {
				const key = getTriggerKey(t)
				return deploymentStatus[key]?.status !== 'deployed'
			})
			.map((trigger) => ({
				key: getTriggerKey(trigger),
				path: trigger.path,
				kind: 'trigger' as Kind,
				diff: undefined as WorkspaceItemDiff | undefined,
				trigger
			}))
		return [...diffItems, ...triggerItems]
	})

	// --- Fork Triggers ---

	type ForkTrigger = {
		path: string
		triggerKind: TriggerKind
		scriptPath: string
		isFlow: boolean
		enabled?: boolean
		extraLabel?: string
	}

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

	let forkTriggers = $state<ForkTrigger[]>([])
	let triggerToDelete = $state<ForkTrigger | undefined>(undefined)
	let deploymentRequestPanel: DeploymentRequestPanel | undefined = $state(undefined)
	let hasOpenDeploymentRequest = $state(false)

	// Trigger detail drawer refs — one per trigger kind. Each is lazy-mounted
	// on first openEdit() call, so having them all sit here is cheap.
	let scheduleEditor: ScheduleEditor | undefined = $state()
	let routeEditor: RouteEditor | undefined = $state()
	let websocketEditor: WebsocketTriggerEditor | undefined = $state()
	let kafkaEditor: KafkaTriggerEditor | undefined = $state()
	let postgresEditor: PostgresTriggerEditor | undefined = $state()
	let natsEditor: NatsTriggerEditor | undefined = $state()
	let mqttEditor: MqttTriggerEditor | undefined = $state()
	let sqsEditor: SqsTriggerEditor | undefined = $state()
	let gcpEditor: GcpTriggerEditor | undefined = $state()
	let emailEditor: EmailTriggerEditor | undefined = $state()

	function openTriggerDetails(trigger: ForkTrigger) {
		const isFlow = trigger.isFlow
		switch (trigger.triggerKind) {
			case 'schedules':
				scheduleEditor?.openEdit(trigger.path, isFlow)
				break
			case 'routes':
				routeEditor?.openEdit(trigger.path, isFlow)
				break
			case 'websockets':
				websocketEditor?.openEdit(trigger.path, isFlow)
				break
			case 'kafka':
				kafkaEditor?.openEdit(trigger.path, isFlow)
				break
			case 'postgres':
				postgresEditor?.openEdit(trigger.path, isFlow)
				break
			case 'nats':
				natsEditor?.openEdit(trigger.path, isFlow)
				break
			case 'mqtt':
				mqttEditor?.openEdit(trigger.path, isFlow)
				break
			case 'sqs':
				sqsEditor?.openEdit(trigger.path, isFlow)
				break
			case 'gcp':
				gcpEditor?.openEdit(trigger.path, isFlow)
				break
			case 'emails':
				emailEditor?.openEdit(trigger.path, isFlow)
				break
		}
	}

	/** Deployable trigger kinds and their list+delete services */
	const triggerServices = {
		schedules: {
			list: (ws: string) => ScheduleService.listSchedules({ workspace: ws }),
			delete: (ws: string, path: string) => ScheduleService.deleteSchedule({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'schedules',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.enabled,
				extraLabel: item.schedule
			})
		},
		routes: {
			list: (ws: string) => HttpTriggerService.listHttpTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				HttpTriggerService.deleteHttpTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'routes',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled',
				extraLabel: `${(item.http_method ?? 'get').toUpperCase()} ${item.route_path ?? ''}`
			})
		},
		websockets: {
			list: (ws: string) => WebsocketTriggerService.listWebsocketTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				WebsocketTriggerService.deleteWebsocketTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'websockets',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled',
				extraLabel: item.url
			})
		},
		kafka: {
			list: (ws: string) => KafkaTriggerService.listKafkaTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				KafkaTriggerService.deleteKafkaTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'kafka',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled',
				extraLabel: item.topics?.join(', ')
			})
		},
		postgres: {
			list: (ws: string) => PostgresTriggerService.listPostgresTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				PostgresTriggerService.deletePostgresTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'postgres',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled'
			})
		},
		nats: {
			list: (ws: string) => NatsTriggerService.listNatsTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				NatsTriggerService.deleteNatsTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'nats',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled',
				extraLabel: item.subjects?.join(', ')
			})
		},
		mqtt: {
			list: (ws: string) => MqttTriggerService.listMqttTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				MqttTriggerService.deleteMqttTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'mqtt',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled'
			})
		},
		sqs: {
			list: (ws: string) => SqsTriggerService.listSqsTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				SqsTriggerService.deleteSqsTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'sqs',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled',
				extraLabel: item.queue_url
			})
		},
		gcp: {
			list: (ws: string) => GcpTriggerService.listGcpTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				GcpTriggerService.deleteGcpTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'gcp',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled',
				extraLabel: item.topic_id
			})
		},
		emails: {
			list: (ws: string) => EmailTriggerService.listEmailTriggers({ workspace: ws }),
			delete: (ws: string, path: string) =>
				EmailTriggerService.deleteEmailTrigger({ workspace: ws, path }),
			normalize: (item: any): ForkTrigger => ({
				path: item.path,
				triggerKind: 'emails',
				scriptPath: item.script_path,
				isFlow: item.is_flow,
				enabled: item.mode === 'enabled',
				extraLabel: getEmailAddress(
					item.local_part,
					item.workspaced_local_part,
					currentWorkspaceId,
					emailDomain ?? ''
				)
			})
		}
	} as const

	let emailDomain = $state<string | undefined>(undefined)

	async function fetchAllTriggers() {
		try {
			emailDomain = await getEmailDomain()
			const entries = Object.values(triggerServices)
			const results = await Promise.allSettled(
				entries.map(async (svc) => {
					const items = await svc.list(currentWorkspaceId)
					return items.map(svc.normalize)
				})
			)
			forkTriggers = results.flatMap((r) => (r.status === 'fulfilled' ? r.value : []))
		} catch (e) {
			console.error('Failed to fetch fork triggers:', e)
			forkTriggers = []
		}
	}

	function deleteTrigger(trigger: ForkTrigger) {
		triggerToDelete = trigger
	}

	async function confirmDeleteTrigger() {
		const trigger = triggerToDelete
		if (!trigger) return
		triggerToDelete = undefined
		const triggerType = triggerKindToTriggerType(trigger.triggerKind)
		const displayName = triggerType ? triggerDisplayNamesMap[triggerType] : trigger.triggerKind
		try {
			const svc = triggerServices[trigger.triggerKind as keyof typeof triggerServices]
			if (!svc) {
				throw new Error(`No service for trigger kind: ${trigger.triggerKind}`)
			}
			await svc.delete(currentWorkspaceId, trigger.path)
			forkTriggers = forkTriggers.filter(
				(t) => !(t.path === trigger.path && t.triggerKind === trigger.triggerKind)
			)
			sendUserToast(`Deleted ${displayName} trigger '${trigger.path}'`)
		} catch (e: any) {
			sendUserToast(`Failed to delete trigger '${trigger.path}': ${e.body || e.message}`, true)
		}
	}

	function getTriggerDisplayName(triggerKind: TriggerKind): string {
		const triggerType = triggerKindToTriggerType(triggerKind)
		return triggerType ? triggerDisplayNamesMap[triggerType] : triggerKind
	}

	// Fetch triggers when workspace is available
	$effect(() => {
		if (currentWorkspaceId) {
			fetchAllTriggers()
		}
	})
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
				items={deployableItems}
				{selectedItems}
				{deploymentStatus}
				selectablePredicate={(item) =>
					item.trigger != null || selectableDiffs.some((d) => getItemKey(d) === item.key)}
				{allSelected}
				onToggleItem={(item) => toggleKey(item.key)}
				onSelectAll={selectAll}
				onDeselectAll={deselectAll}
				emptyMessage="No comparison data available"
			>
				{#snippet header()}
					<div class="flex items-center justify-between bg-surface-tertiary">
						<div class="flex flex-col gap-2 w-full pb-4 border-b">
							<div class="flex flex-wrap gap-1 items-center">
								<ToggleButtonGroup
									disabled={deploying}
									selected="deploy_to"
									onSelected={toggleDeploymentDirection}
									noWFull
								>
									{#snippet children({ item })}
										<ToggleButton
											value="deploy_to"
											label="Deploy to {parentWorkspaceId}"
											icon={ArrowUp}
											{item}
										/>
										<ToggleButton value="update" label="Update current" icon={ArrowDown} {item} />
									{/snippet}
								</ToggleButtonGroup>
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
					{#if !comparison.all_ahead_items_visible || !comparison.all_behind_items_visible}
						<Alert
							title="This fork has changes not visible to your user"
							type="warning"
							class="my-2"
						>
							{#if !comparison.all_ahead_items_visible && !comparison.all_behind_items_visible}
								This fork is ahead and behind its parent
							{:else if !comparison.all_behind_items_visible}
								This fork is behind of its parent
							{:else if !comparison.all_ahead_items_visible}
								This fork is ahead of its parent
							{/if}
							and some of the changes are not visible by you. Only a user with access to the whole context
							may deploy or update this fork. You can share the link to this page to someone with proper
							permissions to get it deployed.
						</Alert>
					{/if}
				{/snippet}

				{#snippet itemSummary(item)}
					{#if item.trigger}
						{@const t = item.trigger as ForkTrigger}
						<span class="text-emphasis">{getTriggerDisplayName(t.triggerKind)}</span>
						<span class="text-secondary mx-1">&rarr;</span>
						<span class="text-secondary">{t.scriptPath}</span>
					{:else}
						{@const diff = item.diff as WorkspaceItemDiff}
						{@const key = item.key}
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
						{#if oldSummary != newSummary && isSelectable && existsInBothWorkspaces}
							<span class="line-through text-secondary">{oldSummary || diff.path}</span>
							{newSummary || diff.path}
						{:else if !existsInBothWorkspaces}
							{newSummary || oldSummary || diff.path}
						{:else}
							{newSummary || diff.path}
						{/if}
					{/if}
				{/snippet}

				{#snippet itemActions(item)}
					{#if item.trigger}
						{@const t = item.trigger as ForkTrigger}
						{@const key = item.key}
						<Badge color="indigo" size="xs">Fork-only</Badge>
						{#if t.isFlow}
							<Badge color="blue" size="xs">flow</Badge>
						{/if}
						{#if t.extraLabel}
							<span class="text-tertiary text-xs">({t.extraLabel})</span>
						{/if}
						{#if t.enabled != null}
							<Badge color={t.enabled ? 'green' : 'gray'} size="xs">
								{t.enabled ? 'Enabled' : 'Disabled'}
							</Badge>
						{/if}
						{#if !deploymentStatus[key] || deploymentStatus[key].status != 'deployed'}
							<Button
								size="xs"
								variant="subtle"
								startIcon={{ icon: Eye }}
								onclick={() => openTriggerDetails(t)}
							>
								Details
							</Button>
							<Button size="xs" variant="subtle" color="red" onclick={() => deleteTrigger(t)}>
								<Trash2 size={12} />
							</Button>
						{/if}
					{:else}
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
									size="xs"
									variant="subtle"
									onclick={() => showDiff(diff.kind as Kind, diff.path)}
								>
									<DiffIcon class="w-3 h-3" />
									Show diff
								</Button>
							</div>
						{/if}
					{/if}
				{/snippet}

				{#snippet footer()}
					<div class="flex items-center justify-between">
						<div></div>

						<div class="flex flex-col items-end gap-2">
							{#if comparison.all_behind_items_visible && comparison.all_ahead_items_visible}
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
											hasUnselectedOnBehalfOf}
										loading={deploying}
										on:click={deployChanges}
									>
										{mergeIntoParent ? 'Deploy' : 'Update'}
										{selectedItems.length} Item{selectedItems.length !== 1 ? 's' : ''}
										{#if selectedConflicts != 0}
											({selectedConflicts} conflicts)
										{/if}
									</Button>
								</div>
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

		<div class="bg-surface-tertiary p-4 rounded-md border">
			<DatatableSchemaDiff {currentWorkspaceId} {parentWorkspaceId} />
		</div>
	</div>

	<DiffDrawer bind:this={diffDrawer} {isFlow} />

	<ScheduleEditor bind:this={scheduleEditor} />
	<RouteEditor bind:this={routeEditor} />
	<WebsocketTriggerEditor bind:this={websocketEditor} />
	<KafkaTriggerEditor bind:this={kafkaEditor} />
	<PostgresTriggerEditor bind:this={postgresEditor} />
	<NatsTriggerEditor bind:this={natsEditor} />
	<MqttTriggerEditor bind:this={mqttEditor} />
	<SqsTriggerEditor bind:this={sqsEditor} />
	<GcpTriggerEditor bind:this={gcpEditor} />
	<EmailTriggerEditor bind:this={emailEditor} />

	<ConfirmationModal
		title="Delete trigger"
		confirmationText="Delete"
		open={!!triggerToDelete}
		onConfirmed={confirmDeleteTrigger}
		onCanceled={() => (triggerToDelete = undefined)}
	>
		{#if triggerToDelete}
			Are you sure you want to delete the {getTriggerDisplayName(triggerToDelete.triggerKind)} trigger
			'{triggerToDelete.path}'?
		{/if}
	</ConfirmationModal>
{:else}
	<div class="flex items-center justify-center h-full">
		<div class="text-gray-500">No comparison data available</div>
	</div>
{/if}
