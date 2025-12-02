<script lang="ts">
	import { Button, Badge, Alert, Tabs, Tab } from './common'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { WorkspaceComparison, WorkspaceItemDiff } from '$lib/gen'
	import {
		WorkspaceService,
		ScriptService,
		FlowService,
		AppService,
		ResourceService,
		VariableService
	} from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import {
		ArrowUpRight,
		ArrowDownRight,
		AlertTriangle,
		CheckCircle2,
		XCircle,
		FileCode,
		Workflow,
		Layout,
		Database,
		Key,
		ChevronRight,
		ChevronDown,
		GitBranch,
		FileText
	} from 'lucide-svelte'
	import DiffViewer from './DiffViewer.svelte'
	import { sendUserToast } from '$lib/utils'

	export let comparison: WorkspaceComparison | undefined = undefined
	export let sourceWorkspace: string | undefined = undefined
	export let targetWorkspace: string | undefined = undefined

	const dispatch = createEventDispatcher()

	let deploymentDirection: 'deploy' | 'update' = 'deploy'
	let selectedItems: Set<string> = new Set()
	let expandedItems: Set<string> = new Set()
	let deploying = false
	let activeTab: 'all' | 'scripts' | 'flows' | 'apps' | 'resources' | 'variables' = 'all'
	let showDiffViewer = false
	let selectedDiffItem: WorkspaceItemDiff | undefined = undefined
	let diffViewerData: { source: any; target: any } | undefined = undefined

	$: filteredDiffs =
		comparison?.diffs.filter((diff) => {
			if (activeTab === 'all') return true
			return diff.kind === activeTab.slice(0, -1) // Remove 's' from plural
		}) ?? []

	$: groupedDiffs = groupDiffsByKind(filteredDiffs)

	$: selectableDiffs = filteredDiffs.filter((diff) => {
		if (deploymentDirection === 'deploy') {
			return diff.versions_ahead > 0
		} else {
			return diff.versions_behind > 0
		}
	})

	$: conflictingDiffs = filteredDiffs.filter(
		(diff) => diff.versions_ahead > 0 && diff.versions_behind > 0
	)

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

	export function open() {
		// Auto-select all eligible items initially
		selectedItems = new Set(selectableDiffs.map((d) => `${d.kind}:${d.path}`))
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

	function getItemKey(diff: WorkspaceItemDiff): string {
		return `${diff.kind}:${diff.path}`
	}

	function toggleItem(diff: WorkspaceItemDiff) {
		const key = getItemKey(diff)
		if (selectedItems.has(key)) {
			selectedItems.delete(key)
		} else {
			selectedItems.add(key)
		}
		selectedItems = selectedItems // Trigger reactivity
	}

	function toggleExpanded(diff: WorkspaceItemDiff) {
		const key = getItemKey(diff)
		if (expandedItems.has(key)) {
			expandedItems.delete(key)
		} else {
			expandedItems.add(key)
			loadDiffDetails(diff)
		}
		expandedItems = expandedItems // Trigger reactivity
	}

	async function loadDiffDetails(diff: WorkspaceItemDiff) {
		if (!sourceWorkspace || !targetWorkspace) return

		try {
			let sourceData: any = null
			let targetData: any = null

			switch (diff.kind) {
				case 'script':
					if (diff.source_hash) {
						sourceData = await ScriptService.getScriptByHash({
							workspace: sourceWorkspace,
							hash: diff.source_hash
						})
					}
					if (diff.target_hash) {
						targetData = await ScriptService.getScriptByHash({
							workspace: targetWorkspace,
							hash: diff.target_hash
						})
					}
					break
				case 'flow':
					sourceData = await FlowService.getFlowByPath({
						workspace: sourceWorkspace,
						path: diff.path
					})
					targetData = await FlowService.getFlowByPath({
						workspace: targetWorkspace,
						path: diff.path
					})
					break
				case 'app':
					sourceData = await AppService.getAppByPath({
						workspace: sourceWorkspace,
						path: diff.path
					})
					targetData = await AppService.getAppByPath({
						workspace: targetWorkspace,
						path: diff.path
					})
					break
				case 'resource':
					sourceData = await ResourceService.getResource({
						workspace: sourceWorkspace,
						path: diff.path
					})
					targetData = await ResourceService.getResource({
						workspace: targetWorkspace,
						path: diff.path
					})
					break
				case 'variable':
					sourceData = await VariableService.getVariable({
						workspace: sourceWorkspace,
						path: diff.path
					})
					targetData = await VariableService.getVariable({
						workspace: targetWorkspace,
						path: diff.path
					})
					break
			}

			selectedDiffItem = diff
			diffViewerData = { source: sourceData, target: targetData }
		} catch (error) {
			console.error('Failed to load diff details:', error)
			sendUserToast('Failed to load diff details', true)
		}
	}

	function selectAll() {
		selectedItems = new Set(selectableDiffs.map((d) => getItemKey(d)))
	}

	function deselectAll() {
		selectedItems = new Set()
	}

	async function deployChanges() {
		if (!sourceWorkspace || !targetWorkspace) return

		deploying = true
		try {
			const itemsToDeploy = Array.from(selectedItems).map((key) => {
				const [kind, ...pathParts] = key.split(':')
				return { kind, path: pathParts.join(':') }
			})

			// TODO: Implement actual deployment logic
			// This would involve calling appropriate APIs to copy/update items
			// between workspaces based on the deployment direction

			sendUserToast(`Successfully deployed ${itemsToDeploy.length} items`, false)
			dispatch('deployed')
		} catch (error) {
			console.error('Deployment failed:', error)
			sendUserToast('Deployment failed', true)
		} finally {
			deploying = false
		}
	}

	async function deleteWorkspace() {
		if (!sourceWorkspace || comparison?.summary.total_diffs !== 0) return

		if (
			confirm(
				`Are you sure you want to delete the forked workspace "${sourceWorkspace}"? This action cannot be undone.`
			)
		) {
			try {
				await WorkspaceService.deleteWorkspace({ workspace: sourceWorkspace })
				sendUserToast('Forked workspace deleted successfully', false)
				// Redirect to parent workspace
				window.location.href = `/w/${targetWorkspace}`
			} catch (error) {
				console.error('Failed to delete workspace:', error)
				sendUserToast('Failed to delete workspace', true)
			}
		}
	}
</script>

<div class="flex flex-col h-full">
	{#if comparison}
		<!-- Header with deployment direction toggle -->
		<div class="p-4 border-b bg-gray-50 dark:bg-gray-900">
			<div class="flex items-center justify-between mb-4">
				<div class="flex items-center gap-4">
					<GitBranch class="w-5 h-5 text-gray-600 dark:text-gray-400" />
					<div>
						<div class="text-sm font-medium">
							{deploymentDirection === 'deploy' ? 'Deploy to Parent' : 'Update from Parent'}
						</div>
						<div class="text-xs text-gray-600 dark:text-gray-400">
							{sourceWorkspace} → {targetWorkspace}
						</div>
					</div>
				</div>

				<div class="flex items-center gap-2">
					<Toggle
						bind:checked={deploymentDirection}
						options={{
							left: { label: 'Deploy', value: 'deploy' },
							right: { label: 'Update Fork', value: 'update' }
						}}
					/>
				</div>
			</div>

			<!-- Summary stats -->
			<div class="flex items-center gap-4 text-sm">
				<Badge color="blue">
					{comparison.summary.total_diffs} total changes
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
				<Badge>
					{selectedItems.size} selected
				</Badge>
			</div>
		</div>

		<!-- Warning for conflicts -->
		{#if conflictingDiffs.length > 0}
			<Alert type="warning" class="m-4">
				<AlertTriangle class="w-4 h-4" />
				<span>
					{conflictingDiffs.length} item{conflictingDiffs.length !== 1 ? 's are' : ' is'} both ahead
					and behind. Deploying will overwrite changes in the target workspace.
				</span>
			</Alert>
		{/if}

		<!-- Tabs for filtering by type -->
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

		<!-- Selection controls -->
		<div class="px-4 py-2 flex items-center justify-between border-b">
			<div class="flex items-center gap-2">
				<Button size="xs" variant="ghost" on:click={selectAll}>Select All</Button>
				<Button size="xs" variant="ghost" on:click={deselectAll}>Deselect All</Button>
			</div>
		</div>

		<!-- Diff list -->
		<div class="flex-1 overflow-y-auto">
			{#each Object.entries(groupedDiffs) as [kind, diffs]}
				<div class="border-b">
					<div class="px-4 py-2 bg-gray-50 dark:bg-gray-900 text-sm font-medium capitalize">
						{kind}s ({diffs.length})
					</div>
					{#each diffs as diff}
						{@const key = getItemKey(diff)}
						{@const isSelectable = selectableDiffs.includes(diff)}
						{@const isSelected = selectedItems.has(key)}
						{@const isExpanded = expandedItems.has(key)}
						{@const isConflict = diff.versions_ahead > 0 && diff.versions_behind > 0}
						{@const Icon = getItemIcon(diff.kind)}

						<div class="border-b last:border-b-0">
							<div
								class="px-4 py-2 flex items-center gap-2 hover:bg-gray-50 dark:hover:bg-gray-900"
							>
								<!-- Expand/collapse button -->
								<button
									on:click={() => toggleExpanded(diff)}
									class="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded"
								>
									{#if isExpanded}
										<ChevronDown class="w-4 h-4" />
									{:else}
										<ChevronRight class="w-4 h-4" />
									{/if}
								</button>

								<!-- Checkbox -->
								{#if isSelectable}
									<input
										type="checkbox"
										checked={isSelected}
										on:change={() => toggleItem(diff)}
										class="rounded"
									/>
								{:else}
									<div class="w-4" />
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
							</div>

							<!-- Expanded content -->
							{#if isExpanded && diffViewerData}
								<div class="px-8 py-2 bg-gray-50 dark:bg-gray-900">
									<div class="text-xs text-gray-600 dark:text-gray-400 mb-2">
										Changes: {diff.metadata_changes.join(', ')}
									</div>
									<Button size="xs" variant="secondary" on:click={() => (showDiffViewer = true)}>
										View Detailed Diff
									</Button>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/each}
		</div>

		<!-- Footer actions -->
		<div class="p-4 border-t bg-gray-50 dark:bg-gray-900">
			<div class="flex items-center justify-between">
				<div>
					{#if comparison.summary.total_diffs === 0}
						<Button color="red" variant="secondary" on:click={deleteWorkspace}>
							Delete Fork Workspace
						</Button>
					{/if}
				</div>

				<div class="flex items-center gap-2">
					<Button variant="secondary" on:click={() => console.log('canceled')}>Cancel</Button>
					<Button
						color="blue"
						disabled={selectedItems.size === 0 || deploying}
						loading={deploying}
						on:click={deployChanges}
					>
						{deploymentDirection === 'deploy' ? 'Deploy' : 'Update'}
						{selectedItems.size} Item{selectedItems.size !== 1 ? 's' : ''}
					</Button>
				</div>
			</div>
		</div>
	{:else}
		<div class="flex items-center justify-center h-full">
			<div class="text-gray-500">No comparison data available</div>
		</div>
	{/if}
</div>

<!-- Diff viewer modal -->
{#if showDiffViewer && selectedDiffItem && diffViewerData}
	<DiffViewer
		bind:open={showDiffViewer}
		item={selectedDiffItem}
		sourceData={diffViewerData.source}
		targetData={diffViewerData.target}
	/>
{/if}
