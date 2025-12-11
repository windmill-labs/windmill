<script lang="ts">
	import {
		AlertTriangle,
		ArrowDown,
		ArrowDownRight,
		ArrowRight,
		ArrowUp,
		ArrowUpRight,
		Building,
		Database,
		DiffIcon,
		FileCode,
		FileText,
		GitFork,
		Key,
		Layout,
		Loader2,
		Workflow
	} from 'lucide-svelte'
	import { Alert, Badge } from './common'
	import {
		AppService,
		FlowService,
		FolderService,
		ResourceService,
		ScriptService,
		VariableService,
		WorkspaceService,
		type WorkspaceComparison,
		type WorkspaceItemDiff
	} from '$lib/gen'
	import Button from './common/button/Button.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import { getAllModules } from './flows/flowExplorer'
	import { userWorkspaces, workspaceStore } from '$lib/stores'

	import {
		existsTrigger,
		getTriggerDependency,
		getTriggersDeployData,
		getTriggerValue,
		type AdditionalInformation,
		type Kind
	} from '$lib/utils_deployable'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Row from './common/table/Row.svelte'
	import { sendUserToast } from '$lib/toast'
	import Tooltip from './Tooltip.svelte'
	import { sleep } from '$lib/utils'
	import { deepEqual } from 'fast-equals'

	interface Props {
		currentWorkspaceId: string
		parentWorkspaceId: string
		comparison: WorkspaceComparison | undefined
	}

	let { currentWorkspaceId, parentWorkspaceId, comparison }: Props = $props()

	let currentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == currentWorkspaceId))
	let parentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == parentWorkspaceId))

	let activeTab = $state<'all' | 'scripts' | 'flows' | 'apps' | 'resources' | 'variables'>('all')
	let mergeIntoParent = $state(true)
	let deploying = $state(false)
	let hasAutoSelected = $state(false)

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

	let groupedDiffs = $derived(groupDiffsByKind(comparison?.diffs ?? []))

	// Summary cache: stores summaries from both workspaces
	type SummaryCache = Record<string, { current?: string; parent?: string; loading?: boolean }>
	let summaryCache = $state<SummaryCache>({})

	function getItemKey(diff: WorkspaceItemDiff): string {
		return `${diff.kind}:${diff.path}`
	}

	function getItemIcon(kind: string) {
		switch (kind) {
			case 'script':
				return FileCode
			case 'flow':
				return Workflow
			case 'app':
				return Layout
			case 'resource':
				return Database
			case 'variable':
				return Key
			default:
				return FileText
		}
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
			}
		} catch (error) {
			console.error(`Failed to fetch summary for ${kind}:${path}`, error)
		}
		return undefined
	}

	async function fetchSummaries(diffs: WorkspaceItemDiff[]) {
		// Only fetch summaries for scripts, flows, and apps
		const itemsToFetch = diffs.filter((diff) => ['script', 'flow', 'app'].includes(diff.kind))

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

	function getSummaryDisplay(key: string): { summary: string; isDifferent: boolean } {
		const cached = summaryCache[key]

		if (!cached || cached.loading) {
			return { summary: '', isDifferent: false }
		}

		const currentSummary = cached.current || ''
		const parentSummary = cached.parent || ''

		const isDifferent =
			currentSummary !== parentSummary && (currentSummary.length > 0 || parentSummary.length > 0)

		// Return the summary from the current workspace by default
		// The visual indicator will show if they're different
		return {
			summary: currentSummary || parentSummary || '',
			isDifferent
		}
	}

	async function getValue(kind: Kind, path: string, workspace: string) {
		try {
			if (kind == 'flow') {
				const flow = await FlowService.getFlowByPath({
					workspace: workspace,
					path: path
				})
				getAllModules(flow.value.modules).forEach((x) => {
					if (x.value.type == 'script' && x.value.hash != undefined) {
						x.value.hash = undefined
					}
				})
				return { summary: flow.summary, description: flow.description, value: flow.value }
			} else if (kind == 'script') {
				const script = await ScriptService.getScriptByPath({
					workspace: workspace,
					path: path
				})
				return {
					content: script.content,
					lock: script.lock,
					schema: script.schema,
					summary: script.summary,
					language: script.language
				}
			} else if (kind == 'app') {
				const app = await AppService.getAppByPath({
					workspace: workspace,
					path: path
				})
				return app
			} else if (kind == 'variable') {
				const variable = await VariableService.getVariable({
					workspace: workspace,
					path: path,
					decryptSecret: true
				})
				return variable.value
			} else if (kind == 'resource') {
				const resource = await ResourceService.getResource({
					workspace: workspace,
					path: path
				})
				return resource.value
			} else if (kind == 'resource_type') {
				const resource = await ResourceService.getResourceType({
					workspace: workspace,
					path: path
				})
				return resource.schema
			} else if (kind == 'raw_app') {
				throw new Error('Raw app deploy not implemented yet')
				// const app = await RawAppService.getRawAppData({
				// 	workspace: workspace,
				// 	path: path
				// })
				// if (alreadyExists) {
				// }
				// await RawAppService.updateRawApp({
				// 	workspace: workspace,
				// 	path: path,
				// 	requestBody: {
				// 		path: path
				// 	}
				// })
			} else if (kind == 'folder') {
				const folder = await FolderService.getFolder({
					workspace: workspace,
					name: path
				})
				return {
					name: folder.name
				}
				// } else if (kind == 'trigger') {
				// 	if (additionalInformation?.triggers) {
				// 		return await getTriggerValue(additionalInformation.triggers.kind, path, workspace)
				// 	} else {
				// 		throw new Error(`Missing trigger information`)
				// 	}
			} else {
				throw new Error(`Unknown kind ${kind}`)
			}
		} catch {
			return {}
		}
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
				getValue(kind, path, workspaceTo),
				getValue(kind, path, workspaceFrom)
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
		selectedItems = selectableDiffs.map((d) => getItemKey(d))
	}

	function deselectAll() {
		selectedItems = []
	}

	async function selectAllNonConflicts() {
		selectedItems = selectableDiffs
			.filter((d) => !(d.ahead > 0 && d.behind > 0))
			.map((d) => getItemKey(d))
	}

	async function checkAlreadyExists(kind: Kind, path: string, workspace: string): Promise<boolean> {
		let exists: boolean
		if (kind == 'flow') {
			exists = await FlowService.existsFlowByPath({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'script') {
			exists = await ScriptService.existsScriptByPath({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'app') {
			exists = await AppService.existsApp({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'raw_app') {
			exists = await RawAppService.existsRawApp({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'variable') {
			exists = await VariableService.existsVariable({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'resource') {
			exists = await ResourceService.existsResource({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'schedule') {
			exists = await ScheduleService.existsSchedule({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'resource_type') {
			exists = await ResourceService.existsResourceType({
				workspace: workspace,
				path: path
			})
		} else if (kind == 'folder') {
			exists = await FolderService.existsFolder({
				workspace: workspace,
				name: path
			})
		} else if (kind === 'trigger') {
			const triggersKind: TriggerKind[] = [
				'kafka',
				'mqtt',
				'nats',
				'postgres',
				'routes',
				'schedules',
				'sqs',
				'websockets',
				'gcp'
			]
			if (
				additionalInformation?.triggers &&
				triggersKind.includes(additionalInformation.triggers.kind)
			) {
				exists = await existsTrigger(
					{ workspace: workspace, path },
					additionalInformation.triggers.kind
				)
			} else {
				throw new Error(
					`Unexpected triggers kind, expected one of: '${triggersKind.join(', ')}' got: ${
						additionalInformation?.triggers?.kind
					}`
				)
			}
		} else {
			throw new Error(`Unknown kind ${kind}`)
		}
		return exists
	}

	const deploymentStatus: Record<
		string,
		{ status: 'loading' | 'deployed' | 'failed'; error?: string }
	> = $state({})

	async function deploy(
		kind: Kind,
		path: string,
		workspaceToDeployTo: string,
		workspaceFrom: string
	) {
		const statusPath = `${kind}:${path}`
		deploymentStatus[statusPath] = { status: 'loading' }

		// await sleep(1000)
		// if (Math.random() > 0.5) {
		// 	deploymentStatus[statusPath] = {status: 'failed'}
		// } else {
		// 	deploymentStatus[statusPath] = {status: 'deployed'}
		// }
		// return

		try {
			let alreadyExists = await checkAlreadyExists(kind, path, workspaceToDeployTo)
			if (kind == 'flow') {
				const flow = await FlowService.getFlowByPath({
					workspace: workspaceFrom,
					path: path
				})
				getAllModules(flow.value.modules).forEach((x) => {
					if (x.value.type == 'script' && x.value.hash != undefined) {
						x.value.hash = undefined
					}
				})
				if (alreadyExists) {
					await FlowService.updateFlow({
						workspace: workspaceToDeployTo,
						path: path,
						requestBody: {
							...flow
						}
					})
				} else {
					await FlowService.createFlow({
						workspace: workspaceToDeployTo,
						requestBody: {
							...flow
						}
					})
				}
			} else if (kind == 'script') {
				const script = await ScriptService.getScriptByPath({
					workspace: workspaceFrom,
					path: path
				})
				await ScriptService.createScript({
					workspace: workspaceToDeployTo,
					requestBody: {
						...script,
						lock: script.lock,
						parent_hash: alreadyExists
							? (
									await ScriptService.getScriptByPath({
										workspace: workspaceToDeployTo,
										path: path
									})
								).hash
							: undefined
					}
				})
			} else if (kind == 'app') {
				const app = await AppService.getAppByPath({
					workspace: workspaceFrom,
					path: path
				})
				if (alreadyExists) {
					await AppService.updateApp({
						workspace: workspaceToDeployTo,
						path: path,
						requestBody: {
							...app
						}
					})
				} else {
					await AppService.createApp({
						workspace: workspaceToDeployTo,
						requestBody: {
							...app
						}
					})
				}
			} else if (kind == 'variable') {
				const variable = await VariableService.getVariable({
					workspace: workspaceFrom,
					path: path,
					decryptSecret: true
				})
				if (alreadyExists) {
					await VariableService.updateVariable({
						workspace: workspaceToDeployTo,
						path: path,
						requestBody: {
							path: path,
							value: variable.value ?? '',
							is_secret: variable.is_secret,
							description: variable.description ?? ''
						},
						alreadyEncrypted: false
					})
				} else {
					await VariableService.createVariable({
						workspace: workspaceToDeployTo,
						requestBody: {
							path: path,
							value: variable.value ?? '',
							is_secret: variable.is_secret,
							description: variable.description ?? ''
						}
					})
				}
			} else if (kind == 'resource') {
				const resource = await ResourceService.getResource({
					workspace: workspaceFrom,
					path: path
				})
				if (alreadyExists) {
					await ResourceService.updateResource({
						workspace: workspaceToDeployTo,
						path: path,
						requestBody: {
							path: path,
							value: resource.value ?? '',
							description: resource.description ?? ''
						}
					})
				} else {
					await ResourceService.createResource({
						workspace: workspaceToDeployTo,
						requestBody: {
							path: path,
							value: resource.value ?? '',
							resource_type: resource.resource_type,
							description: resource.description ?? ''
						}
					})
				}
			} else if (kind == 'resource_type') {
				const resource = await ResourceService.getResourceType({
					workspace: workspaceFrom,
					path: path
				})
				if (alreadyExists) {
					await ResourceService.updateResourceType({
						workspace: workspaceToDeployTo,
						path: path,
						requestBody: {
							schema: resource.schema,
							description: resource.description ?? ''
						}
					})
				} else {
					await ResourceService.createResourceType({
						workspace: workspaceToDeployTo,
						requestBody: {
							description: resource.description ?? '',
							schema: resource.schema,
							name: resource.name
						}
					})
				}
			} else if (kind == 'raw_app') {
				throw new Error('Raw app deploy not implemented yet')
				// const app = await RawAppService.getRawAppData({
				// 	workspace: workspaceFrom,
				// 	path: path
				// })
				// if (alreadyExists) {
				// }
				// await RawAppService.updateRawApp({
				// 	workspace: workspaceFrom,
				// 	path: path,
				// 	requestBody: {
				// 		path: path
				// 	}
				// })
			} else if (kind == 'folder') {
				await FolderService.createFolder({
					workspace: workspaceToDeployTo,
					requestBody: {
						name: path
					}
				})
				// } else if (kind === 'trigger') {
				// 	if (additionalInformation?.triggers) {
				// 		const { data, createFn, updateFn } = await getTriggersDeployData(
				// 			additionalInformation.triggers.kind,
				// 			path,
				// 			workspaceFrom
				// 		)
				// 		if (alreadyExists) {
				// 			await updateFn({
				// 				path,
				// 				workspace: workspaceToDeployTo,
				// 				requestBody: data
				// 			} as any)
				// 		} else {
				// 			await createFn({
				// 				workspace: workspaceToDeployTo,
				// 				requestBody: data
				// 			} as any)
				// 		}
				// 	} else {
				// 		throw new Error('Missing triggers kind')
				// 	}
			} else {
				throw new Error(`Unknown kind ${kind}`)
			}

			// allAlreadyExists[statusPath] = true
			deploymentStatus[statusPath] = { status: 'deployed' }
		} catch (e) {
			deploymentStatus[statusPath] = { status: 'failed', error: e.body || e.message }
			sendUserToast(`Failed to deploy ${statusPath}: ${e.body || e.message}`)
		}
	}

	let deploymentErrorMessage = $state('')
	let allowBehindChangesOverride = $state(false)

	async function isComparisonUpToDate(): Promise<boolean> {
		try {
			const result = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: currentWorkspaceId
			})

			if (!deepEqual(comparison, result)) {
				deploymentErrorMessage = 'New changes detected. Please reload the page and review these before deploying'
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
				await deploy(diff.kind, diff.path, parent, current)
			} else {
				await deploy(diff.kind, diff.path, current, parent)
			}
		}
		deploying = false
		deselectAll()
	}

	function groupDiffsByKind(diffs: WorkspaceItemDiff[]) {
		const grouped: Record<string, WorkspaceItemDiff[]> = {}
		for (const diff of diffs) {
			if (!grouped[diff.kind]) {
				grouped[diff.kind] = []
			}
			grouped[diff.kind].push(diff)
		}
		return grouped
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

	// Fetch summaries when comparison data loads
	$effect(() => {
		if (comparison?.diffs) {
			fetchSummaries(comparison.diffs)
		}
	})

	// Auto-select items on initial load
	$effect(() => {
		if (comparison?.diffs && !hasAutoSelected && selectableDiffs.length > 0) {
			selectDefault()
			hasAutoSelected = true
		}
	})

	async function deleteWorkspace() {
		// if (!sourceWorkspace || comparison?.summary.total_diffs !== 0) return
		//
		// if (
		// 	confirm(
		// 		`Are you sure you want to delete the forked workspace "${sourceWorkspace}"? This action cannot be undone.`
		// 	)
		// ) {
		// 	try {
		// 		await WorkspaceService.deleteWorkspace({ workspace: sourceWorkspace })
		// 		sendUserToast('Forked workspace deleted successfully', false)
		// 		// Redirect to parent workspace
		// 		window.location.href = `/w/${targetWorkspace}`
		// 	} catch (error) {
		// 		console.error('Failed to delete workspace:', error)
		// 		sendUserToast('Failed to delete workspace', true)
		// 	}
		// }
	}
</script>

<div class="flex flex-col h-full">
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
		<div class="bg-surface">
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
								<ToggleButton value="update" label="Update" icon={ArrowDown} {item} />
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
					<!-- {/if} -->
				</div>
			</div>
		</div>

		{#if conflictingDiffs.length > 0}
			<Alert title="Conflicting changes detected" type="warning" class="mt-2">
				<!-- <AlertTriangle class="w-4 h-4" /> -->
				<span>
					{conflictingDiffs.length} item{conflictingDiffs.length !== 1 ? 's have' : ' has'} conflicting
					changes, it was modified on the original workspace while changes were made on this fork. Make
					sure to resolve these before merging.
				</span>
			</Alert>
		{/if}

		<div class="px-4 py-2 flex items-center justify-between">
			<div
				class="flex items-center gap-2 text-secondary text-sm"
				class:opacity-50={selectableDiffs.length == 0}
			>
				<input
					type="checkbox"
					disabled={selectableDiffs.length == 0}
					checked={allSelected}
					onchange={allSelected ? deselectAll : selectAll}
					class="rounded max-w-4 w-full"
				/> Select all
			</div>
		</div>

		<div class="flex-1 overflow-y-auto">
			<div class="border rounded-md bg-surface-tertiary">
				{#each comparison.diffs as diff}
					{@const key = getItemKey(diff)}
					{@const isSelectable = selectableDiffs.includes(diff)}
					{@const isSelected = selectedItems.includes(key)}
					{@const isConflict = diff.ahead > 0 && diff.behind > 0}
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

					<Row
						isSelectable={isSelectable && !(deploymentStatus[key]?.status == 'deployed')}
						alignWithSelectable={true}
						disabled={!isSelectable}
						selected={isSelected && !(deploymentStatus[key]?.status == 'deployed')}
						onSelect={() => toggleItem(diff)}
						path={diff.kind != 'resource' && diff.kind != 'variable' ? diff.path : ''}
						marked={undefined}
						kind={diff.kind}
						canFavorite={false}
						workspaceId=""
						starred={false}
					>
						{#snippet customSummary()}
							{#if oldSummary != newSummary && isSelectable && existsInBothWorkspaces}
								<span class="line-through text-secondary">{oldSummary || diff.path}</span>
								{newSummary || diff.path}
							{:else if !existsInBothWorkspaces}
								{newSummary || oldSummary || diff.path}
							{:else}
								{newSummary || diff.path}
							{/if}
						{/snippet}
						{#snippet actions()}
							<!-- Status badges -->
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

									{#if !diff.exists_in_fork && diff.exists_in_source && diff.ahead == 0 && diff.behind > 0}
										<Badge
											title="This item was newly created in the parent workspace '{parentWorkspaceId}'"
											color="indigo"
											size="xs">New</Badge
										>
									{/if}
									{#if !diff.exists_in_fork && diff.exists_in_source && diff.ahead > 0}
										<Badge
											title="This item was deleted in '{currentWorkspaceId}'"
											color="red"
											size="xs">Deleted</Badge
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
							{#if deploymentStatus[key]}
								{#if deploymentStatus[key].status == 'loading'}
									<Loader2 class="animate-spin" />
								{:else if deploymentStatus[key].status == 'deployed'}
									<Badge color="green">Deployed</Badge>
								{:else if deploymentStatus[key].status == 'failed'}
									<div class="inline-flex gap-1">
										<Badge color="red">Failed</Badge>
										<Tooltip>{deploymentStatus[key].error}</Tooltip></div
									>
								{/if}
							{/if}
						{/snippet}
					</Row>
				{/each}
			</div>
			<!-- 	</div> -->
			<!-- {/each} -->
		</div>

		<div class="p-4 bg-surface">
			<div class="flex items-center justify-between">
				<div>
					{#if comparison.summary.total_diffs === 0}
						<Button color="red" variant="accent-secondary" on:click={deleteWorkspace}>
							Delete Fork Workspace
						</Button>
					{/if}
				</div>

				<div class="flex flex-col items-right gap-2">
					<Button
						color="blue"
						disabled={selectedItems.length === 0 || deploying}
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
			</div>
					{#if deploymentErrorMessage != ''}
					<div class="max-w-80 mr-0">
					<Alert title="Cannot deploy these changes" type="error" class="my-2">
						<span>
							{deploymentErrorMessage}
						</span>
					</Alert>
					</div>
					{/if}
		</div>
		<DiffDrawer bind:this={diffDrawer} {isFlow} />
	{:else}
		<div class="flex items-center justify-center h-full">
			<div class="text-gray-500">No comparison data available</div>
		</div>
	{/if}

	<!-- <DeployWorkspaceItems kind="script" initialPath="u/admin/economical_script" workspaceToDeployTo={parentWorkspaceId} /> -->
</div>
