<script lang="ts">
	import {
		AlertTriangle,
		ArrowDown,
		ArrowDownRight,
		ArrowLeft,
		ArrowRight,
		ArrowUp,
		ArrowUpRight,
		Building,
		CodeXml,
		Database,
		DiffIcon,
		FileCode,
		FileText,
		GitBranch,
		GitFork,
		Key,
		Layout,
		Workflow
	} from 'lucide-svelte'
	import { Alert, Badge, Tab, Tabs } from './common'
	import {
		AppService,
		FlowService,
		FolderService,
		ResourceService,
		ScriptService,
		VariableService,
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

	let filteredDiffs = $derived(
		comparison?.diffs.filter((diff) => {
			if (activeTab === 'all') return true
			return diff.kind === activeTab.slice(0, -1) // Remove 's' from plural
		}) ?? []
	)

	let selectableDiffs = $derived(
		comparison?.diffs.filter((diff) => {
			if (mergeIntoParent) {
				return diff.versions_ahead > 0
			} else {
				return diff.versions_behind > 0
			}
		}) ?? []
	)

	let selectedItems = $state<string[]>([])

	let conflictingDiffs = $derived(
		comparison?.diffs.filter((diff) => diff.versions_ahead > 0 && diff.versions_behind > 0) ?? []
	)

	let groupedDiffs = $derived(groupDiffsByKind(comparison?.diffs ?? []))

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
			.filter((d) => !(d.versions_ahead > 0 && d.versions_behind > 0))
			.map((d) => getItemKey(d))
	}

	function deployChanges() {}

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
		// console.log(selectedItems)
		// selectedItems = selectedItems // Trigger reactivity
	}

	function toggleDeploymentDirection(v: string) {
		deselectAll()
		mergeIntoParent = v == 'deploy_to'
		if (mergeIntoParent) {
			selectAll()
		} else {
			selectAllNonConflicts()
		}
	}

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
	;(async () => {
		selectAll()
	})()
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
		<div class="bg-surface">
			<div class="flex items-center justify-between">
				<div
					class="flex flex-col gap-2 border bg-surface-tertiary w-full p-4 border-radius-5 rounded"
				>
					<!-- <div class="text-sm font-medium"> -->
					<!-- 	{mergeIntoParent ? 'Deploying to Parent' : 'Updating from Parent'} ({parentWorkspaceId}) -->
					<!-- </div> -->
					<div class="flex flex-row gap-1 items-center">
						<ToggleButtonGroup selected="deploy_to" onSelected={toggleDeploymentDirection} noWFull>
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
							{selectableDiffs.length} deployable
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

		<!-- <Tabs bind:selected={activeTab} class="px-4 pt-4"> -->
		<!-- 	<Tab value="all" label="All ({comparison.summary.total_diffs})" /> -->
		<!-- 	{#if comparison.summary.scripts_changed > 0} -->
		<!-- 		<Tab value="scripts" label="Scripts ({comparison.summary.scripts_changed})" /> -->
		<!-- 	{/if} -->
		<!-- 	{#if comparison.summary.flows_changed > 0} -->
		<!-- 		<Tab value="flows" label="Flows ({comparison.summary.flows_changed})" /> -->
		<!-- 	{/if} -->
		<!-- 	{#if comparison.summary.apps_changed > 0} -->
		<!-- 		<Tab value="apps" label="Apps ({comparison.summary.apps_changed})" /> -->
		<!-- 	{/if} -->
		<!-- 	{#if comparison.summary.resources_changed > 0} -->
		<!-- 		<Tab value="resources" label="Resources ({comparison.summary.resources_changed})" /> -->
		<!-- 	{/if} -->
		<!-- 	{#if comparison.summary.variables_changed > 0} -->
		<!-- 		<Tab value="variables" label="Variables ({comparison.summary.variables_changed})" /> -->
		<!-- 	{/if} -->
		<!-- </Tabs> -->

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
			<!-- {#each Object.entries(groupedDiffs) as [kind, diffs]} -->
			<!-- 	<div class="border-b"> -->
			<!-- 		<div class="px-4 py-2 bg-surface-secondary text-sm font-medium capitalize"> -->
			<!-- 			{kind}s ({diffs.length}) -->
			<!-- 		</div> -->
			<div class="border rounded-md bg-surface-tertiary">
				{#each comparison.diffs as diff}
					{@const key = getItemKey(diff)}
					{@const isSelectable = selectableDiffs.includes(diff)}
					{@const isSelected = selectedItems.includes(key)}
					{@const isConflict = diff.versions_ahead > 0 && diff.versions_behind > 0}
					{@const Icon = getItemIcon(diff.kind)}

					<Row
						{isSelectable}
						alignWithSelectable={true}
						disabled={!isSelectable}
						selected={isSelected}
						onSelect={() => toggleItem(diff)}
						path={diff.path}
						marked={undefined}
						kind={diff.kind as any}
						canFavorite={false}
						workspaceId=""
						starred={false}
					>
						{#snippet actions()}
							<!-- Status badges -->
							<div class="flex items-center gap-2">
								{#if diff.versions_ahead > 0}
									<Badge color="green" size="xs">
										<ArrowUpRight class="w-3 h-3 inline" />
										{diff.versions_ahead} ahead
									</Badge>
								{/if}
								{#if diff.versions_behind > 0}
									<Badge color="blue" size="xs">
										<ArrowDownRight class="w-3 h-3 inline" />
										{diff.versions_behind} behind
									</Badge>
								{/if}
								{#if isConflict}
									<Badge color="orange" size="xs">
										<AlertTriangle class="w-3 h-3 inline" />
										Conflict
									</Badge>
								{/if}
								{#if diff.metadata_changes.includes('only_in_source')}
									<Badge color="gray" size="xs">New</Badge>
								{/if}
								{#if diff.metadata_changes.includes('only_in_target')}
									<Badge color="gray" size="xs">Deleted</Badge>
								{/if}
							</div>
							<Button
								size="xs"
								variant="subtle"
								onclick={() => showDiff(diff.kind as Kind, diff.path)}
							>
								<DiffIcon class="w-3 h-3" />
								Show diff
							</Button>
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

				<div class="flex items-center gap-2">
					<Button
						color="blue"
						disabled={selectedItems.length === 0 || deploying}
						loading={deploying}
						on:click={deployChanges}
					>
						{mergeIntoParent ? 'Deploy' : 'Update'}
						{selectedItems.length} Item{selectedItems.length !== 1 ? 's' : ''}
					</Button>
				</div>
			</div>
		</div>
		<DiffDrawer bind:this={diffDrawer} {isFlow} />
	{:else}
		<div class="flex items-center justify-center h-full">
			<div class="text-gray-500">No comparison data available</div>
		</div>
	{/if}

	<!-- <DeployWorkspaceItems kind="script" initialPath="u/admin/economical_script" workspaceToDeployTo={parentWorkspaceId} /> -->
</div>
