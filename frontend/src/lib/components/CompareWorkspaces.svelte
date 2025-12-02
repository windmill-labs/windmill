<script lang="ts">
	import {
		AlertTriangle,
		ArrowDownRight,
		ArrowUpRight,
		Database,
		DiffIcon,
		FileCode,
		FileText,
		GitBranch,
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
	import { userWorkspaces } from '$lib/stores'
	import WorkspaceCard from './WorkspaceCard.svelte'

	import {
		existsTrigger,
		getTriggerDependency,
		getTriggersDeployData,
		getTriggerValue,
		type AdditionalInformation,
		type Kind
	} from '$lib/utils_deployable'

	interface Props {
		currentWorkspaceId: string
		parentWorkspaceId: string
		comparison: WorkspaceComparison | undefined
	}

	let { currentWorkspaceId, parentWorkspaceId, comparison }: Props = $props()

	let currentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == currentWorkspaceId))
	let parentWorkspaceInfo = $derived($userWorkspaces.find((w) => w.id == parentWorkspaceId))

	let activeTab = $state<'all' | 'scripts' | 'flows' | 'apps' | 'resources' | 'variables'>('all')
	let selectedItems = $state<string[]>([])
	let mergeIntoParent = $state(true)
	let deploying = $state(false)

	let filteredDiffs = $derived(
		comparison?.diffs.filter((diff) => {
			if (activeTab === 'all') return true
			return diff.kind === activeTab.slice(0, -1) // Remove 's' from plural
		}) ?? []
	)

	let selectableDiffs = $derived(
		filteredDiffs.filter((diff) => {
			if (mergeIntoParent) {
				return diff.versions_ahead > 0
			} else {
				return diff.versions_behind > 0
			}
		})
	)

	let conflictingDiffs = $derived(
		filteredDiffs.filter((diff) => diff.versions_ahead > 0 && diff.versions_behind > 0)
	)

	let groupedDiffs = $derived(groupDiffsByKind(filteredDiffs))
	let flexRowDirections = $derived(mergeIntoParent ? 'flex-row' : 'flex-row-reverse')

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

	function selectAll() {
		selectedItems = selectableDiffs.map((d) => getItemKey(d))
	}

	function deselectAll() {
		selectedItems = []
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
	{#if comparison}
		<div class="p-4 border-b bg-surface">
			<div class="flex items-center justify-between mb-4">
				<div class="flex items-center gap-4">
					<GitBranch class="w-5 h-5 text-gray-600 dark:text-gray-400" />
					<div>
						<div class="text-sm font-medium">
							{mergeIntoParent ? 'Deploying to Parent' : 'Updating from Parent'} ({parentWorkspaceId})
						</div>
						{#if mergeIntoParent}
							<div class="text-xs text-gray-600 dark:text-gray-400">
								{currentWorkspaceId} → {parentWorkspaceId}
							</div>
						{:else}
							<div class="text-xs text-gray-600 dark:text-gray-400">
								{parentWorkspaceId} → {currentWorkspaceId}
							</div>
						{/if}
					</div>
				</div>

				<div class="flex items-center gap-2">
					<div class="flex items-center justify-center {flexRowDirections}">
						<div class="flex items-center gap-3">
							<div class="bg-green-100 rounded-md px-4 py-2">
								<WorkspaceCard workspace={currentWorkspaceInfo} isFork={true} className="text-green-800 font-semibold"/>

								<!-- <span class="text-green-800 font-semibold">{currentWorkspaceId}</span> -->
							</div>
						</div>

						<button
							id="arrowButton"
							title="toggle deployment direction"
							class="flex items-center gap-2 mx-6 cursor-pointer hover:opacity-70 transition-opacity group"
							onclick={() => (mergeIntoParent = !mergeIntoParent)}
						>
							<svg
								class="w-6 h-6 text-gray-400 group-hover:hidden transition-all"
								fill="currentColor"
								viewBox="0 0 20 20"
							>
								<path
									fill-rule="evenodd"
									d="M10.293 3.293a1 1 0 011.414 0l6 6a1 1 0 010 1.414l-6 6a1 1 0 01-1.414-1.414L14.586 11H3a1 1 0 110-2h11.586l-4.293-4.293a1 1 0 010-1.414z"
									clip-rule="evenodd"
								/>
							</svg>
							<svg
								class="w-6 h-6 text-gray-600 hidden group-hover:block transition-all"
								fill="none"
								stroke="currentColor"
								viewBox="0 0 24 24"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"
								/>
							</svg>
						</button>

						<div class="flex items-center gap-3">
							<div class="bg-blue-100 rounded-md px-4 py-2">
								<WorkspaceCard workspace={parentWorkspaceInfo} isFork={false} className="text-green-800 font-semibold"/>
								<!-- <span class="text-blue-800 font-semibold">{parentWorkspaceId}</span> -->
							</div>
						</div>
					</div>
				</div>
			</div>
			<div class="flex items-center gap-4 text-sm">
				<Badge color="blue">
					{comparison.summary.total_diffs} total items
				</Badge>
				{#if conflictingDiffs.length > 0}
					<Badge color="orange">
						<AlertTriangle class="w-3 h-3 inline mr-1" />
						{conflictingDiffs.length} conflicts
					</Badge>
				{/if}
				<Badge color="green">
					{selectableDiffs.length} deployable
				</Badge>
			</div>
		</div>

		{#if conflictingDiffs.length > 0}
			<Alert title="Conflicting changes detected" type="warning" class="m-4">
				<!-- <AlertTriangle class="w-4 h-4" /> -->
				<span>
					{conflictingDiffs.length} item{conflictingDiffs.length !== 1 ? 's are' : ' is'} both ahead
					and behind. Deploying could overwrite older changes.
				</span>
			</Alert>
		{/if}

		<Tabs bind:selected={activeTab} class="px-4 pt-4">
			<Tab value="all" label="All ({comparison.summary.total_diffs})" />
			{#if comparison.summary.scripts_changed > 0}
				<Tab value="scripts" label="Scripts ({comparison.summary.scripts_changed})" />
			{/if}
			{#if comparison.summary.flows_changed > 0}
				<Tab value="flows" label="Flows ({comparison.summary.flows_changed})" />
			{/if}
			{#if comparison.summary.apps_changed > 0}
				<Tab value="apps" label="Apps ({comparison.summary.apps_changed})" />
			{/if}
			{#if comparison.summary.resources_changed > 0}
				<Tab value="resources" label="Resources ({comparison.summary.resources_changed})" />
			{/if}
			{#if comparison.summary.variables_changed > 0}
				<Tab value="variables" label="Variables ({comparison.summary.variables_changed})" />
			{/if}
		</Tabs>

		<div class="px-4 py-2 flex items-center justify-between border-b">
			<div class="flex items-center gap-2">
				<Button size="xs" variant="subtle" on:click={selectAll}>Select All</Button>
				<Button size="xs" variant="subtle" on:click={deselectAll}>Deselect All</Button>
			</div>
		</div>

		<div class="flex-1 overflow-y-auto">
			{#each Object.entries(groupedDiffs) as [kind, diffs]}
				<div class="border-b">
					<div class="px-4 py-2 bg-surface-secondary text-sm font-medium capitalize">
						{kind}s ({diffs.length})
					</div>
					{#each diffs as diff}
						{@const key = getItemKey(diff)}
						{@const isSelectable = selectableDiffs.includes(diff)}
						{@const isSelected = selectedItems.includes(key)}
						{@const isConflict = diff.versions_ahead > 0 && diff.versions_behind > 0}
						{@const Icon = getItemIcon(diff.kind)}

						<div class="border-b last:border-b-0">
							<div
								class:opacity-50={!isSelectable}
								class:bg-surface-hover={isSelected}
								class="px-4 py-2 flex items-center gap-2 hover:bg-hover"
							>
								<!-- Expand/collapse button -->

								<!-- Checkbox -->
								{#if isSelectable}
									<input
										type="checkbox"
										checked={isSelected}
										onchange={() => toggleItem(diff)}
										class="rounded max-w-4"
									/>
								{:else}
									<div class="w-4"></div>
								{/if}

								<!-- Icon -->
								<Icon class="w-4 h-4 text-gray-500" />

								<!-- Path -->
								<span class="flex-1 font-mono text-sm">{diff.path}</span>

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
									onclick={() => showDiff(kind as Kind, diff.path)}
								>
									<DiffIcon class="w-3 h-3" />
									Show diff
								</Button>
							</div>

							<!-- Expanded content -->
							<!-- {#if isExpanded && diffViewerData} -->
							<!-- 	<div class="px-8 py-2 bg-gray-50 dark:bg-gray-900"> -->
							<!-- 		<div class="text-xs text-gray-600 dark:text-gray-400 mb-2"> -->
							<!-- 			Changes: {diff.metadata_changes.join(', ')} -->
							<!-- 		</div> -->
							<!-- 		<Button size="xs" variant="secondary" on:click={() => (showDiffViewer = true)}> -->
							<!-- 			View Detailed Diff -->
							<!-- 		</Button> -->
							<!-- 	</div> -->
							<!-- {/if} -->
						</div>
					{/each}
				</div>
			{/each}
		</div>

		<div class="p-4 border-t bg-surface">
			<div class="flex items-center justify-between">
				<div>
					{#if comparison.summary.total_diffs === 0}
						<Button color="red" variant="accent-secondary" on:click={deleteWorkspace}>
							Delete Fork Workspace
						</Button>
					{/if}
				</div>

				<div class="flex items-center gap-2">
					<Button variant="accent-secondary" on:click={() => console.log('canceled')}>Cancel</Button
					>
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
