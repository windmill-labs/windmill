<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Alert from './common/alert/Alert.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import {
		ChevronRight,
		ChevronDown,
		FileText,
		GitBranch,
		Layers,
		ArrowRight,
		Package
	} from 'lucide-svelte'

	interface DependencyNode {
		path: string
		kind: 'script' | 'flow' | 'app'
		nodeIds?: string[]
		children?: DependencyNode[]
		expanded?: boolean
		loading?: boolean
		childrenCount?: number
	}

	let {
		importedPath,
		title = 'Dependency Warning',
		confirmText = 'Proceed Anyway',
		onConfirm,
		onCancel,
		open = true,
		isUnnamedDefault = false,
		language = undefined
	}: {
		importedPath: string
		title?: string
		confirmText?: string
		onConfirm: () => void
		onCancel: () => void
		open?: boolean
		isUnnamedDefault?: boolean
		language?: string
	} = $props()

	let dependencies: DependencyNode[] = $state([])
	let loading = $state(true)

	// Load initial dependencies
	async function loadInitialDependencies() {
		try {
			loading = true
			console.log('Loading dependents for:', importedPath)
			const dependents = await WorkspaceService.getDependents({
				workspace: $workspaceStore!,
				importedPath
			})

			console.log('Found dependents:', dependents)

			if (dependents.length === 0) {
				dependencies = []
				return
			}

			// Get counts for all dependents to see which have children
			const dependentPaths = dependents.map(dep => dep.importer_path)
			const amounts = await WorkspaceService.getDependentsAmounts({
				workspace: $workspaceStore!,
				requestBody: dependentPaths
			})

			console.log('Dependents amounts:', amounts)

			const amountMap = new Map(amounts.map(a => [a.imported_path, a.count]))

			dependencies = dependents.map(dep => ({
				path: dep.importer_path,
				kind: dep.importer_kind as 'script' | 'flow' | 'app',
				nodeIds: dep.importer_node_ids ?? undefined,
				expanded: false,
				childrenCount: amountMap.get(dep.importer_path) || 0
			}))
		} catch (error) {
			console.error('Error loading dependencies:', error)
			dependencies = []
		} finally {
			loading = false
		}
	}

	// Load children for a specific dependency
	async function loadChildren(node: DependencyNode) {
		if (node.children || node.loading) return

		try {
			node.loading = true
			const childImportedPath = node.path // Use the dependency path directly
			const dependents = await WorkspaceService.getDependents({
				workspace: $workspaceStore!,
				importedPath: childImportedPath
			})

			// Get counts for child dependents
			const dependentPaths = dependents.map(dep => dep.importer_path)
			const amounts = await WorkspaceService.getDependentsAmounts({
				workspace: $workspaceStore!,
				requestBody: dependentPaths
			})

			const amountMap = new Map(amounts.map(a => [a.imported_path, a.count]))

			node.children = dependents.map(dep => ({
				path: dep.importer_path,
				kind: dep.importer_kind as 'script' | 'flow' | 'app',
				nodeIds: dep.importer_node_ids ?? undefined,
				expanded: false,
				childrenCount: amountMap.get(dep.importer_path) || 0
			}))
		} catch (error) {
			console.error('Error loading children:', error)
			node.children = []
		} finally {
			node.loading = false
		}
	}

	function getIcon(kind: string) {
		switch (kind) {
			case 'script':
				return FileText
			case 'flow':
				return GitBranch
			case 'app':
				return Layers
			default:
				return FileText
		}
	}

	function getKindLabel(kind: string) {
		return kind.charAt(0).toUpperCase() + kind.slice(1)
	}

	async function toggleExpand(node: DependencyNode) {
		if (!node.expanded && !node.children) {
			await loadChildren(node)
		}
		node.expanded = !node.expanded
	}

	function getTotalDependentsCount(): number {
		function countNode(node: DependencyNode): number {
			let count = 1
			if (node.children) {
				count += node.children.reduce((sum, child) => sum + countNode(child), 0)
			}
			return count
		}
		return dependencies.reduce((sum, dep) => sum + countNode(dep), 0)
	}

	// Load dependencies when component mounts (skip for unnamed/default dependencies)
	$effect(() => {
		if ($workspaceStore && importedPath && !isUnnamedDefault) {
			loadInitialDependencies()
		} else if (isUnnamedDefault) {
			// For unnamed dependencies, skip loading - just show warning
			loading = false
		}
	})

	function handleCancel() {
		onCancel()
	}

	async function handleConfirm() {
		await onConfirm()
	}
</script>

<ConfirmationModal
	{open}
	{title}
	confirmationText={confirmText}
	type={isUnnamedDefault ? 'danger' : undefined}
	onConfirmed={handleConfirm}
	onCanceled={handleCancel}
>
	{#if loading}
		<div class="flex items-center gap-2">
			<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600"></div>
			<span class="text-sm">Loading dependencies...</span>
		</div>
	{:else if isUnnamedDefault}
		<!-- For unnamed/default dependencies, just show the critical warning -->
		<div class="space-y-3">
			<p class="font-bold">This will redeploy ALL existing {language} scripts and flows/apps that have {language} steps without explicit named dependencies!</p>
			<p>Default (unnamed) dependencies are automatically used by any {language} runnable that doesn't specify a named dependency. Changing this affects your entire workspace.</p>
			<div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 border-l-4 border-gray-400 dark:border-gray-600 mt-3">
				<div class="flex items-center gap-2 mb-2">
					<Package size={16} class="text-gray-600 dark:text-gray-400" />
					<span class="font-medium text-gray-800 dark:text-gray-200">
						Default Workspace Dependencies
					</span>
				</div>
				<div class="font-mono text-sm text-gray-700 dark:text-gray-300 ml-6">
					{importedPath}
				</div>
			</div>
		</div>
	{:else}
		<!-- For named dependencies, show the dependents tree -->
		<div class="space-y-4">
			<!-- Header with workspace dependencies info -->
			<div class="bg-yellow-50 dark:bg-yellow-900/20 rounded-lg p-3 border-l-4 border-yellow-400">
				<div class="flex items-center gap-2 mb-2">
					<Package size={16} class="text-yellow-600 dark:text-yellow-400" />
					<span class="font-medium text-yellow-800 dark:text-yellow-200">Workspace Dependencies</span>
				</div>
				<div class="font-mono text-sm text-yellow-700 dark:text-yellow-300 ml-6">
					{importedPath}
				</div>
			</div>

			{#if dependencies.length === 0}
				<Alert type="info" title="No Dependent Runnables Found">
					{#snippet children()}
						<p class="text-sm">No dependent runnables were found for these workspace dependencies, but the action will still proceed.</p>
					{/snippet}
				</Alert>
			{:else}
				<!-- Arrow pointing down -->
				<div class="flex justify-center">
					<ArrowRight size={16} class="text-yellow-600 dark:text-yellow-400 transform rotate-90" />
				</div>

				<!-- Summary -->
				<div class="bg-yellow-50 dark:bg-yellow-900/30 rounded p-3">
					<p class="text-sm text-yellow-800 dark:text-yellow-200">
						This action will trigger redeployment of <strong>{getTotalDependentsCount()}</strong>
						{getTotalDependentsCount() === 1 ? 'dependent runnable' : 'dependent runnables'}:
					</p>
				</div>

				<!-- Dependency tree -->
				<div class="space-y-1 max-h-64 overflow-y-auto">
					{#each dependencies as dependency}
						{@render DependencyNode({ node: dependency, level: 0 })}
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</ConfirmationModal>

{#snippet DependencyNode({ node, level }: { node: DependencyNode, level: number })}
	{@const Icon = getIcon(node.kind)}
	<div style="margin-left: {level * 1}rem;">
		<div class="flex items-center gap-2 p-2 bg-white dark:bg-gray-800 rounded border hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
			<!-- Expand/collapse button -->
			{#if (node.childrenCount ?? 0) > 0}
				<button
					onclick={() => toggleExpand(node)}
					class="flex-shrink-0 p-1 hover:bg-gray-200 dark:hover:bg-gray-600 rounded"
					disabled={node.loading}
				>
					{#if node.loading}
						<div class="animate-spin rounded-full h-3 w-3 border-b-2 border-gray-400"></div>
					{:else if node.expanded}
						<ChevronDown size={12} class="text-gray-500" />
					{:else}
						<ChevronRight size={12} class="text-gray-500" />
					{/if}
				</button>
			{:else}
				<div class="w-5"></div>
			{/if}

			<!-- Kind icon and label -->
			<div class="flex items-center gap-1.5 flex-shrink-0">
				<Icon size={14} class="text-blue-600 dark:text-blue-400" />
				<span class="text-xs font-medium text-blue-700 dark:text-blue-300 uppercase tracking-wide">
					{getKindLabel(node.kind)}
				</span>
			</div>

			<!-- Path -->
			<div class="flex-1 min-w-0">
				<span class="font-mono text-sm text-gray-900 dark:text-gray-100 truncate block">
					{node.path}
				</span>
				{#if node.nodeIds && node.nodeIds.length > 0}
					<span class="text-xs text-gray-500 dark:text-gray-400">
						{node.nodeIds.length} node{node.nodeIds.length !== 1 ? 's' : ''}
					</span>
				{/if}
			</div>

			<!-- Children count -->
			{#if (node.childrenCount ?? 0) > 0}
				<span class="flex-shrink-0 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 px-2 py-1 rounded-full">
					{node.childrenCount} dep{node.childrenCount !== 1 ? 's' : ''}
				</span>
			{/if}
		</div>

		<!-- Children -->
		{#if node.expanded && node.children}
			<div class="ml-2 border-l border-gray-200 dark:border-gray-600">
				{#each node.children as child}
					{@render DependencyNode({ node: child, level: level + 1 })}
				{/each}
			</div>
		{/if}
	</div>
{/snippet}
