<script lang="ts">
	import {
		AlertTriangle,
		ArrowDown,
		ArrowDownRight,
		ArrowRight,
		ArrowUp,
		ArrowUpRight,
		Building,
		DiffIcon,
		GitFork,
		Loader2,
		Trash2,
		Upload
	} from 'lucide-svelte'
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
	import Row from './common/table/Row.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import DeployWorkspaceDrawer from './DeployWorkspaceDrawer.svelte'
	import ParentWorkspaceProtectionAlert from './ParentWorkspaceProtectionAlert.svelte'
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'

	import type { Kind } from '$lib/utils_deployable'
	import { deployItem, getItemValue, getOnBehalfOfEmail } from '$lib/utils_workspace_deploy'
	import Tooltip from './Tooltip.svelte'
	import OnBehalfOfSelector, {
		needsOnBehalfOfSelection,
		type OnBehalfOfChoice
	} from './OnBehalfOfSelector.svelte'
	import { sendUserToast } from '$lib/toast'
	import { deepEqual } from 'fast-equals'
	import WorkspaceDeployLayout from './WorkspaceDeployLayout.svelte'
	import type { TriggerKind } from './triggers'
	import { triggerDisplayNamesMap, triggerKindToTriggerType } from './triggers/utils'
	import { getEmailAddress, getEmailDomain } from './triggers/email/utils'
	import { base } from '$lib/base'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'

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
			} else if (kind === 'app') {
				const app = await AppService.getAppByPath({ workspace, path })
				return app.summary
			} else if (kind === 'folder') {
				const folder = await FolderService.getFolder({ workspace, name: path.slice(2) })
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
			['script', 'flow', 'app', 'folder'].includes(diff.kind)
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
		const flowsAndScripts = diffs.filter((d) => ['flow', 'script', 'app'].includes(d.kind))
		for (const diff of flowsAndScripts) {
			for (const workspace of [currentWorkspaceId, parentWorkspaceId]) {
				const workspacedKey = getWorkspacedKey(workspace, getItemKey(diff))
				if (onBehalfOfInfo[workspacedKey] !== undefined) continue

				try {
					onBehalfOfInfo[workspacedKey] = await getOnBehalfOfEmail(
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

	// Get source workspace email for an item
	function getSourceEmail(itemKey: string): string | undefined {
		const sourceWorkspace = mergeIntoParent ? currentWorkspaceId : parentWorkspaceId
		return onBehalfOfInfo[getWorkspacedKey(sourceWorkspace, itemKey)]
	}

	// Get target workspace email for an item (existing item in destination)
	function getTargetEmail(itemKey: string): string | undefined {
		const targetWorkspace = mergeIntoParent ? parentWorkspaceId : currentWorkspaceId
		return onBehalfOfInfo[getWorkspacedKey(targetWorkspace, itemKey)]
	}

	// Check if an item needs on_behalf_of selection (more than 1 unique option)
	function itemNeedsOnBehalfOfSelection(itemKey: string, kind: string): boolean {
		return needsOnBehalfOfSelection(
			kind,
			getSourceEmail(itemKey),
			getTargetEmail(itemKey),
			$userStore?.email
		)
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

	// Get the email to use for deployment based on user's choice
	function getOnBehalfOfEmailForDeploy(itemKey: string): string | undefined {
		const choice = onBehalfOfChoice[itemKey]
		if (choice === 'source') return getSourceEmail(itemKey)
		if (choice === 'target') return getTargetEmail(itemKey)
		// 'me' or undefined = don't pass, backend will use deploying user's email
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

	let allSelected = $derived(selectedItems.length == selectableDiffs.length)

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
		statusPath: string
	) {
		deploymentStatus[statusPath] = { status: 'loading' }

		const result = await deployItem({
			kind,
			path,
			workspaceFrom,
			workspaceTo: workspaceToDeployTo,
			onBehalfOfEmail: getOnBehalfOfEmailForDeploy(statusPath)
		})

		if (result.success) {
			deploymentStatus[statusPath] = { status: 'deployed' }
		} else {
			deploymentStatus[statusPath] = { status: 'failed', error: result.error }
			sendUserToast(`Failed to deploy ${statusPath}: ${result.error}`)
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
		for (const itemKey of selectedItems) {
			const diff = selectableDiffs.find((d) => itemKey == getItemKey(d))

			if (!diff) {
				sendUserToast(`Undeployable item: ${itemKey}`, true)
				continue
			}

			if (mergeIntoParent) {
				await deploy(diff.kind as Kind, diff.path, parent, current, itemKey)
			} else {
				await deploy(diff.kind as Kind, diff.path, current, parent, itemKey)
			}
		}
		deploying = false
		deselectAll()
	}

	function toggleItem(diff: WorkspaceItemDiff) {
		const key = getItemKey(diff)
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

	// Transform diffs to deployable item format for the shared layout
	let deployableItems = $derived(
		(comparison?.diffs ?? [])
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
	)

	// --- Fork Triggers ---

	type ForkTrigger = {
		path: string
		triggerKind: TriggerKind
		scriptPath: string
		isFlow: boolean
		enabled?: boolean
		extraLabel?: string
	}

	let forkTriggers = $state<ForkTrigger[]>([])
	let loadingTriggers = $state(true)
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state(undefined)
	let triggerToDelete = $state<ForkTrigger | undefined>(undefined)

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
		loadingTriggers = true
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
		} finally {
			loadingTriggers = false
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

	const triggerKindToPagePath: Record<string, string> = {
		schedules: '/schedules',
		routes: '/routes',
		websockets: '/websocket_triggers',
		kafka: '/kafka_triggers',
		postgres: '/postgres_triggers',
		nats: '/nats_triggers',
		mqtt: '/mqtt_triggers',
		sqs: '/sqs_triggers',
		gcp: '/gcp_triggers',
		emails: '/email_triggers'
	}

	function getTriggerHref(triggerKind: TriggerKind): string | undefined {
		const pagePath = triggerKindToPagePath[triggerKind]
		return pagePath ? `${base}${pagePath}` : undefined
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

	<WorkspaceDeployLayout
		items={deployableItems}
		{selectedItems}
		{deploymentStatus}
		selectablePredicate={(item) => selectableDiffs.some((d) => getItemKey(d) === item.key)}
		{allSelected}
		onToggleItem={(item) => {
			const diff = comparison?.diffs.find((d) => getItemKey(d) === item.key)
			if (diff) toggleItem(diff)
		}}
		onSelectAll={selectAll}
		onDeselectAll={deselectAll}
		emptyMessage="No comparison data available"
	>
		{#snippet header()}
			<div class="flex items-center justify-between">
				<div
					class="flex flex-col gap-2 border bg-surface-tertiary w-full p-4 border-radius-5 rounded"
				>
					<div class="flex flex-row gap-1 items-center">
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
								title={!mergeIntoParent ? currentWorkspaceInfo.name : parentWorkspaceInfo.name}
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
						{/if}
					</div>
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
		{/snippet}

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
				<Alert title="This fork has changes not visible to your user" type="warning" class="my-2">
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
			{@const diff = item.diff as WorkspaceItemDiff}
			{@const key = item.key}
			{@const isSelectable = selectableDiffs.includes(diff)}
			{@const oldSummary = mergeIntoParent ? summaryCache[key]?.parent : summaryCache[key]?.current}
			{@const newSummary = mergeIntoParent ? summaryCache[key]?.current : summaryCache[key]?.parent}
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
		{/snippet}

		{#snippet itemActions(item)}
			{@const diff = item.diff as WorkspaceItemDiff}
			{@const key = item.key}
			{@const sourceEmail = getSourceEmail(key)}
			{@const targetEmail = getTargetEmail(key)}
			{@const isConflict = diff.ahead > 0 && diff.behind > 0}
			{@const existsInBothWorkspaces = !(
				(diff.exists_in_fork && !diff.exists_in_source) ||
				(!diff.exists_in_fork && diff.exists_in_source)
			)}
			<!-- On-behalf-of selector -->
			{#if itemNeedsOnBehalfOfSelection(key, diff.kind)}
				<OnBehalfOfSelector
					{sourceEmail}
					{targetEmail}
					selected={onBehalfOfChoice[key]}
					onSelect={(choice) => (onBehalfOfChoice[key] = choice)}
					kind={diff.kind}
					canPreserve={canPreserveOnBehalfOf}
				/>
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
					<Button size="xs" variant="subtle" onclick={() => showDiff(diff.kind as Kind, diff.path)}>
						<DiffIcon class="w-3 h-3" />
						Show diff
					</Button>
				</div>
			{/if}
		{/snippet}

		{#snippet footer()}
			<div class="flex items-center justify-between">
				<div></div>

				<div class="flex flex-col items-end gap-2">
					{#if comparison.all_behind_items_visible && comparison.all_ahead_items_visible}
						{#if !(mergeIntoParent && !canDeployToParent)}
							<Button
								color="blue"
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
							{#if hasUnselectedOnBehalfOf}
								<span class="text-xs text-yellow-600">
									You must set the "on behalf of" user for all items before deploying
									<Tooltip class="text-yellow-600">
										The "run on behalf of" field defines which user's permissions will be applied
										during execution. Make sure this is set to an appropriate user before deploying.
									</Tooltip>
								</span>
							{/if}
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

	<!-- Fork Triggers Section -->
	<div class="mt-6">
		<div class="flex items-center gap-2 mb-2">
			<h3 class="text-sm font-semibold">Triggers created in this fork</h3>
			{#if !loadingTriggers}
				<Badge color="indigo" size="xs"
					>{forkTriggers.length} trigger{forkTriggers.length !== 1 ? 's' : ''}</Badge
				>
			{/if}
		</div>

		<Alert title="Deploy and/or delete these triggers" type="info" class="mb-2">
			When forking a workspace, triggers are not forked to avoid unnecessary executions or
			collisions. If you created this triggers with the intention of deploying them to the parent
			workspace, you can do so here. Otherwise it is recommended to delete them or disable them.
		</Alert>

		{#if loadingTriggers}
			<div class="flex items-center gap-2 text-secondary text-sm p-4">
				<Loader2 class="animate-spin w-4 h-4" />
				Loading triggers...
			</div>
		{:else if forkTriggers.length === 0}
			<div class="text-secondary text-sm p-4 border rounded-md bg-surface-tertiary">
				No triggers in this fork workspace.
			</div>
		{:else}
			<div class="border rounded-md bg-surface-tertiary">
				{#each forkTriggers as trigger (trigger.triggerKind + ':' + trigger.path)}
					<Row
						kind="trigger"
						triggerKind={trigger.triggerKind}
						path={trigger.path}
						href={getTriggerHref(trigger.triggerKind)}
						marked={undefined}
						isSelectable={false}
						canFavorite={false}
						workspaceId={currentWorkspaceId}
					>
						{#snippet customSummary()}
							<span>{getTriggerDisplayName(trigger.triggerKind)}</span>
							<span class="text-secondary mx-1">&rarr;</span>
							<span class="text-secondary">{trigger.scriptPath}</span>
							{#if trigger.isFlow}
								<Badge color="blue" size="xs">flow</Badge>
							{/if}
							{#if trigger.extraLabel}
								<span class="text-tertiary text-xs">({trigger.extraLabel})</span>
							{/if}
						{/snippet}
						{#snippet actions()}
							{#if trigger.enabled != null}
								<Badge color={trigger.enabled ? 'green' : 'gray'} size="xs">
									{trigger.enabled ? 'Enabled' : 'Disabled'}
								</Badge>
							{/if}
							<Button
								size="xs"
								variant="subtle"
								onclick={() => {
									deploymentDrawer?.openDrawer(trigger.path, 'trigger', {
										triggers: { kind: trigger.triggerKind }
									})
								}}
							>
								<Upload size={12} />
								Deploy
							</Button>
							<Button size="xs" variant="subtle" color="red" onclick={() => deleteTrigger(trigger)}>
								<Trash2 size={12} />
							</Button>
						{/snippet}
					</Row>
				{/each}
			</div>
		{/if}
	</div>

	<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
	<DiffDrawer bind:this={diffDrawer} {isFlow} />
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
