<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Skeleton } from '$lib/components/common'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import WorkspaceDependenciesEditor from '$lib/components/WorkspaceDependenciesEditor.svelte'
	import DependencyWarning from '$lib/components/DependencyWarning.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { workspaceStore, userStore } from '$lib/stores'
	import { Plus, FileText, Search, Code2, Edit, Eye } from 'lucide-svelte'
	import { WorkspaceDependenciesService, WorkspaceService } from '$lib/gen'
	import { untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import type uFuzzy from '@leeoniya/ufuzzy'

	// RFC #7105: Global workspace dependencies structure
	interface WorkspaceDependencies {
		id?: number
		name: string | null // null means unnamed/workspace default
		content?: string
		language?: string
		workspace_id?: string
		created_at: string
		archived?: boolean
		description?: string // Added description field
		canWrite: boolean
		marked?: string
	}

	let filter = $state('')
	let workspaceDependencies: WorkspaceDependencies[] | undefined = $state()
	let filteredItems: (WorkspaceDependencies & { marked?: string })[] | undefined = $state()
	let workspaceDependenciesEditor: WorkspaceDependenciesEditor | undefined = $state()
	
	// View modal state
	let viewDrawer: Drawer | undefined = $state()
	let viewContent: string = $state('')
	let viewLanguage: string = $state('python')
	let viewPath: string = $state('')

	// Dependency warning state
	let showDependencyWarning = $state(false)
	let pendingAction: (() => Promise<void>) | null = $state(null)
	let currentImportedPath: string | null = $state(null)
	let warningTitle = $state('')
	let warningConfirmText = $state('')

	// Mock search options similar to Home page
	const opts: uFuzzy.Options = {
		sort: (info, haystack, needle) => {
			let {
				idx,
				chars,
				terms,
				interLft2,
				interLft1,
				start,
				intraIns,
				interIns
			} = info

			const cmp = new Intl.Collator('en').compare

			const sortResult = idx
				.map((v, i) => i)
				.sort(
					(ia, ib) =>
						// most contig chars matched
						chars[ib] - chars[ia] ||
						// least char intra-fuzz (most contiguous)
						intraIns[ia] - intraIns[ib] ||
						// most prefix bounds, boosted by full term matches
						terms[ib] +
							interLft2[ib] +
							0.5 * interLft1[ib] -
							(terms[ia] + interLft2[ia] + 0.5 * interLft1[ia]) ||
						// highest density of match (least term inter-fuzz)
						interIns[ia] - interIns[ib] ||
						// earliest start of match
						start[ia] - start[ib] ||
						// alphabetic
						cmp(haystack[idx[ia]], haystack[idx[ib]])
				)
			return sortResult
		}
	}

	let languages = $derived(
		Array.from(
			new Set(filteredItems?.map((x) => x.language || 'python3') ?? [])
		).sort()
	)

	let languageFilter: string | undefined = $state(undefined)

	$effect(() => {
		if ($workspaceStore) {
			languageFilter = undefined
		}
	})

	let preFilteredItems = $derived(
		languageFilter == undefined
			? workspaceDependencies
			: workspaceDependencies?.filter((x) => (x.language || 'python3') === languageFilter)
	)

	// Load workspace dependencies using actual API
	async function loadWorkspaceDependencies(): Promise<void> {
		try {
			const apiRequirements = await WorkspaceDependenciesService.listWorkspaceDependencies({ 
				workspace: $workspaceStore! 
			})
			
			// Map API response to our interface and add canWrite property
			workspaceDependencies = apiRequirements.map((req: any) => ({
				...req,
				description: req.description || `${req.name || 'Default'} requirements for ${req.language}`,
				canWrite: true // TODO: Implement proper permissions check
			}))
		} catch (error) {
			console.error('Failed to load workspace dependencies:', error)
			// Fallback to mock data on error
			workspaceDependencies = [
				{
					id: 2,
					name: 'typescript-common',
					content: '{\n  "dependencies": {\n    "axios": "^1.6.0",\n    "lodash": "^4.17.21"\n  }\n}',
					language: 'typescript',
					description: 'Common TypeScript dependencies for web APIs',
					workspace_id: $workspaceStore,
					created_at: new Date().toISOString(),
					archived: false,
					canWrite: true
				},
				{
					id: 3,
					name: 'go-admin',
					content: 'module windmill-admin-script\n\ngo 1.21\n\nrequire (\n\tgithub.com/gorilla/mux v1.8.1\n)',
					language: 'go',
					description: 'Go dependencies for admin operations',
					workspace_id: $workspaceStore,
					created_at: new Date().toISOString(),
					archived: false,
					canWrite: true
				},
				{
					id: 1,
					name: null, // Workspace default
					content: 'requests>=2.31.0\npandas>=2.0.0\nnumpy>=1.24.0',
					language: 'python3',
					description: 'Default Python requirements for all scripts',
					workspace_id: $workspaceStore,
					created_at: new Date().toISOString(),
					archived: false,
					canWrite: true
				}
			]
		}
	}

	$effect(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				loadWorkspaceDependencies()
			})
		}
	})

	function createNewWorkspaceDependencies() {
		workspaceDependenciesEditor?.initNew()
	}

	function editWorkspaceDependencies(deps: WorkspaceDependencies) {
		workspaceDependenciesEditor?.editWorkspaceDependencies(deps.id!, deps.name, deps.language!)
	}

	function viewWorkspaceDependencies(deps: WorkspaceDependencies) {
		viewPath = deps.name || `Workspace Default (${deps.language})`
		viewContent = deps.content || ''
		viewLanguage = deps.language || 'python3'
		viewDrawer?.openDrawer()
	}

	function getDisplayName(deps: WorkspaceDependencies): string {
		return deps.name || `Default (${deps.language})`
	}

	function getFullFilename(deps: WorkspaceDependencies): string {
		const extension = getFileExtension(deps.language || 'python3')
		return deps.name ? `${deps.name}.${extension}` : extension
	}

	function getFileExtension(language: string): string {
		switch (language) {
			case 'python':
			case 'python3':
				return 'requirements.in'
			case 'typescript':
				return 'package.json'
			case 'go':
				return 'go.mod'
			case 'php':
				return 'composer.json'
			default:
				return 'requirements.txt'
		}
	}

	// Archive workspace dependencies with dependency check
	async function archiveWorkspaceDependencies(deps: WorkspaceDependencies): Promise<void> {
		const importedPath = getWorkspaceDependenciesPath(deps.name, deps.language || 'python3')
		
		try {
			// Check if there are any dependents
			const dependents = await WorkspaceService.getDependents({
				workspace: $workspaceStore!,
				importedPath
			})

			// Always show warning for archive
			currentImportedPath = importedPath
			warningTitle = `Archive Warning`
			warningConfirmText = 'Archive Anyway'
			pendingAction = () => executeArchive(deps)
			showDependencyWarning = true
		} catch (error) {
			console.error('Error checking dependents:', error)
			// On error, proceed without warning
			await executeArchive(deps)
		}
	}

	async function executeArchive(deps: WorkspaceDependencies): Promise<void> {
		try {
			await WorkspaceDependenciesService.archiveWorkspaceDependencies({
				workspace: $workspaceStore!,
				language: deps.language as any,
				name: deps.name || ''
			})
			sendUserToast(`Archived workspace dependencies: ${getDisplayName(deps)}`)
			loadWorkspaceDependencies() // Reload the list
		} catch (error) {
			console.error('Error archiving workspace dependencies:', error)
			sendUserToast(`Failed to archive workspace dependencies: ${error.message}`, true)
		}
	}

	// Delete workspace dependencies with dependency check
	async function deleteWorkspaceDependencies(deps: WorkspaceDependencies): Promise<void> {
		const importedPath = getWorkspaceDependenciesPath(deps.name, deps.language || 'python3')
		
		try {
			// Check if there are any dependents
			const dependents = await WorkspaceService.getDependents({
				workspace: $workspaceStore!,
				importedPath
			})

			// Always show warning for delete
			currentImportedPath = importedPath
			warningTitle = `Delete Warning`
			warningConfirmText = 'Delete Anyway'
			pendingAction = () => executeDelete(deps)
			showDependencyWarning = true
		} catch (error) {
			console.error('Error checking dependents:', error)
			// On error, proceed without warning
			await executeDelete(deps)
		}
	}

	async function executeDelete(deps: WorkspaceDependencies): Promise<void> {
		try {
			await WorkspaceDependenciesService.deleteWorkspaceDependencies({
				workspace: $workspaceStore!,
				language: deps.language as any,
				name: deps.name || ''
			})
			sendUserToast(`Deleted workspace dependencies: ${getDisplayName(deps)}`)
			loadWorkspaceDependencies() // Reload the list
		} catch (error) {
			console.error('Error deleting workspace dependencies:', error)
			sendUserToast(`Failed to delete workspace dependencies: ${error.message}`, true)
		}
	}

	function viewWorkspaceDependenciesHistory(deps: WorkspaceDependencies): void {
		// TODO: Implement view history functionality
		console.log('View history:', deps)
	}

	async function viewReferencedFrom(deps: WorkspaceDependencies): Promise<void> {
		try {
			const path = getWorkspaceDependenciesPath(deps.name, deps.language || 'python3')
			const dependents = await WorkspaceService.getDependents({
				workspace: $workspaceStore!,
				importedPath: path
			})
			
			if (dependents.length === 0) {
				sendUserToast('No dependent runnables found for these workspace dependencies')
			} else {
				// Show dependents in a modal or navigate to a detailed view
				console.log('Dependents:', dependents)
				sendUserToast(`Found ${dependents.length} dependent runnable${dependents.length !== 1 ? 's' : ''}`)
			}
		} catch (error) {
			console.error('Error fetching dependent runnables:', error)
			sendUserToast('Failed to fetch dependent runnables', true)
		}
	}

	function getWorkspaceDependenciesPath(name: string | null, language: string): string {
		const extension = getFileExtension(language)
		return name ? `workspace_dependencies/${name}.${extension}` : `workspace_dependencies/${extension}`
	}

	async function handleWarningConfirm(): Promise<void> {
		if (pendingAction) {
			showDependencyWarning = false
			await pendingAction()
			pendingAction = null
			currentImportedPath = null
		}
	}

	function handleWarningCancel(): void {
		showDependencyWarning = false
		pendingAction = null
		currentImportedPath = null
	}


	function getLanguageForHighlighting(lang: string): 'python3' | 'nativets' | 'go' | 'php' | undefined {
		// Map our requirement languages to syntax highlighting languages
		switch (lang) {
			case 'python':
				return 'python3'
			case 'typescript':
				return 'nativets'
			case 'go':
				return 'go'
			case 'php':
				return 'php'
			default:
				return undefined
		}
	}
</script>

<WorkspaceDependenciesEditor bind:this={workspaceDependenciesEditor} on:create={loadWorkspaceDependencies} />

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.name || 'Default') + ' ' + (x.language || '') + ' ' + (x.content || '')}
	{opts}
/>

<CenteredPage>
	<PageHeader
		title="Workspace Dependencies"
		tooltip="Workspace Dependencies define dependency specifications for scripts by language. Unnamed dependencies serve as workspace defaults, while named dependencies can be referenced by scripts using #raw_reqs annotations."
		documentationLink="https://www.windmill.dev/docs/"
	>
		<div class="flex flex-row justify-end">
			<Button size="md" startIcon={{ icon: Plus }} on:click={createNewWorkspaceDependencies}>
				New&nbsp;workspace&nbsp;dependencies
			</Button>
		</div>
	</PageHeader>

	<div class="pt-2">
		<div class="relative text-tertiary">
			<input
				placeholder="Search workspace dependencies by name, language, or content..."
				bind:value={filter}
				class="bg-surface !h-10 !px-4 !pr-10 !rounded-lg text-sm focus:outline-none w-full"
			/>
			<button aria-label="Search" type="submit" class="absolute right-0 top-0 mt-3 mr-4">
				<Search class="h-4 w-4" />
			</button>
		</div>
	</div>

	<div class="min-h-[56px]">
		<ListFilters bind:selectedFilter={languageFilter} filters={languages} />
	</div>

	<div class="relative overflow-x-auto pb-40 pr-4">
		{#if !filteredItems}
			<Skeleton layout={[0.5, [2], 1]} />
			{#each new Array(3) as _}
				<Skeleton layout={[[3.5], 0.5]} />
			{/each}
		{:else if filteredItems.length == 0}
			<div class="flex flex-col items-center justify-center h-full py-12">
				<FileText size={48} class="text-secondary mb-4" />
				<div class="text-md font-medium">No workspace dependencies found</div>
				<div class="text-sm text-secondary mb-4">
					Try changing the filters or creating new workspace dependencies
				</div>
				<Button startIcon={{ icon: Plus }} on:click={createNewWorkspaceDependencies}>
					Create your first workspace dependencies
				</Button>
			</div>
		{:else}
			<DataTable size="xs">
				<Head>
					<tr>
						<Cell head first>Name</Cell>
						<Cell head>Language</Cell>
						<Cell head>Description</Cell>
						<Cell head>Type</Cell>
						<Cell head>Created</Cell>
						<Cell head last>Actions</Cell>
					</tr>
				</Head>
				<tbody class="divide-y">
					{#each filteredItems as deps}
						{@const displayName = getDisplayName(deps)}
						{@const fullFilename = getFullFilename(deps)}
						<Row>
							<Cell first>
								<div class="flex items-center gap-2">
									<FileText size={16} class="text-secondary" />
									<div class="flex flex-col">
										<a
											class="break-all hover:text-primary cursor-pointer font-medium"
											onclick={() => editWorkspaceDependencies(deps)}
											title={fullFilename}
										>
											{#if deps.marked}
												{@html deps.marked}
											{:else}
												{displayName}
											{/if}
										</a>
										<span class="text-xs text-tertiary font-mono">{fullFilename}</span>
									</div>
								</div>
							</Cell>
							<Cell>
								<div class="flex items-center gap-1">
									<Code2 size={14} class="text-secondary" />
									<span class="text-xs font-mono text-secondary">
										{deps.language || 'python3'}
									</span>
								</div>
							</Cell>
							<Cell>
								<span class="text-xs text-tertiary" title={deps.description}>
									{deps.description || '-'}
								</span>
							</Cell>
							<Cell>
								<span class="text-xs px-1.5 py-0.5 rounded bg-opacity-50 font-medium"
									class:bg-blue-100="{deps.name === null}"
									class:text-blue-700="{deps.name === null}"
									class:bg-gray-100="{deps.name !== null}"
									class:text-gray-600="{deps.name !== null}"
								>
									{deps.name === null ? 'Default' : 'Named'}
								</span>
							</Cell>
							<Cell>
								<span class="text-xs text-tertiary">
									{new Date(deps.created_at).toLocaleDateString()}
								</span>
							</Cell>
							<Cell last>
								<div class="flex gap-1 flex-wrap">
									<Button size="xs" variant="border" color="light" startIcon={{ icon: Eye }} on:click={() => viewWorkspaceDependencies(deps)}>
										View
									</Button>
									{#if deps.canWrite}
										<Button size="xs" variant="border" color="light" startIcon={{ icon: Edit }} on:click={() => editWorkspaceDependencies(deps)}>
											Edit
										</Button>
										<!-- Placeholder buttons -->
										<Button size="xs" variant="ghost" color="gray" on:click={() => archiveWorkspaceDependencies(deps)} title="Archive">
											Archive
										</Button>
										<Button size="xs" variant="ghost" color="red" on:click={() => deleteWorkspaceDependencies(deps)} title="Delete">
											Delete
										</Button>
										<Button size="xs" variant="ghost" color="gray" on:click={() => viewWorkspaceDependenciesHistory(deps)} title="View History">
											History
										</Button>
										<Button size="xs" variant="ghost" color="gray" on:click={() => viewReferencedFrom(deps)} title="Referenced From">
											Refs
										</Button>
									{/if}
								</div>
							</Cell>
						</Row>
					{/each}
				</tbody>
			</DataTable>
		{/if}
	</div>
</CenteredPage>

<Drawer bind:this={viewDrawer} size="900px">
	<DrawerContent title="View Requirement - {viewPath}" on:close={viewDrawer?.closeDrawer}>
		{#snippet actions()}
			<div class="flex items-center gap-2">
				<Code2 size={16} class="text-secondary" />
				<span class="text-sm font-mono text-secondary">{viewLanguage}</span>
			</div>
		{/snippet}
		
		<div class="space-y-4">
			{#if viewContent}
				<HighlightCode language={getLanguageForHighlighting(viewLanguage)} code={viewContent} />
			{:else}
				<div class="text-center text-secondary py-8">
					<FileText size={48} class="mx-auto mb-4 opacity-50" />
					<p>No content available for this requirement</p>
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>

{#if showDependencyWarning && currentImportedPath}
	<DependencyWarning
		importedPath={currentImportedPath}
		title={warningTitle}
		confirmText={warningConfirmText}
		cancelText="Cancel"
		onConfirm={handleWarningConfirm}
		onCancel={handleWarningCancel}
	/>
{/if}