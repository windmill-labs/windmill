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
	import RequirementEditor from '$lib/components/RequirementEditor.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { workspaceStore, userStore } from '$lib/stores'
	import { Plus, FileText, Search, Code2, Edit, Eye } from 'lucide-svelte'
	import { RawRequirementsService, WorkspaceService } from '$lib/gen'
	import { untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import type uFuzzy from '@leeoniya/ufuzzy'

	// RFC #7105: Global raw requirements structure
	interface Requirement {
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
	let requirements: Requirement[] | undefined = $state()
	let filteredItems: (Requirement & { marked?: string })[] | undefined = $state()
	let requirementEditor: RequirementEditor | undefined = $state()
	
	// View modal state
	let viewDrawer: Drawer | undefined = $state()
	let viewContent: string = $state('')
	let viewLanguage: string = $state('python')
	let viewPath: string = $state('')

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
			? requirements
			: requirements?.filter((x) => (x.language || 'python3') === languageFilter)
	)

	// Load raw requirements using actual API
	async function loadRequirements(): Promise<void> {
		try {
			const apiRequirements = await RawRequirementsService.listRawRequirements({ 
				workspace: $workspaceStore! 
			})
			
			// Map API response to our interface and add canWrite property
			requirements = apiRequirements.map((req: any) => ({
				...req,
				description: req.description || `${req.name || 'Default'} requirements for ${req.language}`,
				canWrite: true // TODO: Implement proper permissions check
			}))
		} catch (error) {
			console.error('Failed to load raw requirements:', error)
			// Fallback to mock data on error
			requirements = [
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
				loadRequirements()
			})
		}
	})

	function createNewRequirement() {
		requirementEditor?.initNew()
	}

	function editRequirement(req: Requirement) {
		requirementEditor?.editRequirement(req.id!, req.name, req.language!)
	}

	function viewRequirement(req: Requirement) {
		viewPath = req.name || `Workspace Default (${req.language})`
		viewContent = req.content || ''
		viewLanguage = req.language || 'python3'
		viewDrawer?.openDrawer()
	}

	function getDisplayName(req: Requirement): string {
		return req.name || `Default (${req.language})`
	}

	function getFullFilename(req: Requirement): string {
		const extension = getFileExtension(req.language || 'python3')
		return req.name ? `${req.name}.${extension}` : extension
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

	// Placeholder functions for future implementation
	async function archiveRequirement(req: Requirement): Promise<void> {
		try {
			await RawRequirementsService.archiveRawRequirements({
				workspace: $workspaceStore!,
				language: req.language as any,
				name: req.name || ''
			})
			sendUserToast(`Archived requirement: ${getDisplayName(req)}`)
			loadRequirements() // Reload the list
		} catch (error) {
			console.error('Error archiving requirement:', error)
			sendUserToast(`Failed to archive requirement: ${error.message}`, true)
		}
	}

	async function deleteRequirement(req: Requirement): Promise<void> {
		try {
			await RawRequirementsService.deleteRawRequirements({
				workspace: $workspaceStore!,
				language: req.language as any,
				name: req.name || ''
			})
			sendUserToast(`Deleted requirement: ${getDisplayName(req)}`)
			loadRequirements() // Reload the list
		} catch (error) {
			console.error('Error deleting requirement:', error)
			sendUserToast(`Failed to delete requirement: ${error.message}`, true)
		}
	}

	function viewHistory(req: Requirement): void {
		// TODO: Implement view history functionality
		console.log('View history:', req)
	}

	async function viewReferencedFrom(req: Requirement): Promise<void> {
		try {
			const path = getRequirementPath(req.name, req.language || 'python3')
			const dependents = await WorkspaceService.getDependents({
				workspace: $workspaceStore!,
				importedPath: path
			})
			
			if (dependents.length === 0) {
				sendUserToast('No dependents found for this requirement')
			} else {
				// Show dependents in a modal or navigate to a detailed view
				console.log('Dependents:', dependents)
				sendUserToast(`Found ${dependents.length} dependent${dependents.length !== 1 ? 's' : ''}`)
			}
		} catch (error) {
			console.error('Error fetching dependents:', error)
			sendUserToast('Failed to fetch dependents', true)
		}
	}

	function getRequirementPath(name: string | null, language: string): string {
		const extension = getFileExtension(language)
		return name ? `raw_requirements/${name}.${extension}` : `raw_requirements/${extension}`
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

<RequirementEditor bind:this={requirementEditor} on:create={loadRequirements} />

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.name || 'Default') + ' ' + (x.language || '') + ' ' + (x.content || '')}
	{opts}
/>

<CenteredPage>
	<PageHeader
		title="Raw Requirements"
		tooltip="Global raw requirements define dependency specifications for scripts by language. Unnamed requirements serve as workspace defaults, while named requirements can be referenced by scripts using #raw_reqs annotations."
		documentationLink="https://www.windmill.dev/docs/"
	>
		<div class="flex flex-row justify-end">
			<Button size="md" startIcon={{ icon: Plus }} on:click={createNewRequirement}>
				New&nbsp;requirement
			</Button>
		</div>
	</PageHeader>

	<div class="pt-2">
		<div class="relative text-tertiary">
			<input
				placeholder="Search raw requirements by name, language, or content..."
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
				<div class="text-md font-medium">No requirements found</div>
				<div class="text-sm text-secondary mb-4">
					Try changing the filters or creating a new requirement
				</div>
				<Button startIcon={{ icon: Plus }} on:click={createNewRequirement}>
					Create your first requirement
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
					{#each filteredItems as req}
						{@const displayName = getDisplayName(req)}
						{@const fullFilename = getFullFilename(req)}
						<Row>
							<Cell first>
								<div class="flex items-center gap-2">
									<FileText size={16} class="text-secondary" />
									<div class="flex flex-col">
										<a
											class="break-all hover:text-primary cursor-pointer font-medium"
											onclick={() => editRequirement(req)}
											title={fullFilename}
										>
											{#if req.marked}
												{@html req.marked}
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
										{req.language || 'python3'}
									</span>
								</div>
							</Cell>
							<Cell>
								<span class="text-xs text-tertiary" title={req.description}>
									{req.description || '-'}
								</span>
							</Cell>
							<Cell>
								<span class="text-xs px-1.5 py-0.5 rounded bg-opacity-50 font-medium"
									class:bg-blue-100="{req.name === null}"
									class:text-blue-700="{req.name === null}"
									class:bg-gray-100="{req.name !== null}"
									class:text-gray-600="{req.name !== null}"
								>
									{req.name === null ? 'Default' : 'Named'}
								</span>
							</Cell>
							<Cell>
								<span class="text-xs text-tertiary">
									{new Date(req.created_at).toLocaleDateString()}
								</span>
							</Cell>
							<Cell last>
								<div class="flex gap-1 flex-wrap">
									<Button size="xs" variant="border" color="light" startIcon={{ icon: Eye }} on:click={() => viewRequirement(req)}>
										View
									</Button>
									{#if req.canWrite}
										<Button size="xs" variant="border" color="light" startIcon={{ icon: Edit }} on:click={() => editRequirement(req)}>
											Edit
										</Button>
										<!-- Placeholder buttons -->
										<Button size="xs" variant="ghost" color="gray" on:click={() => archiveRequirement(req)} title="Archive">
											Archive
										</Button>
										<Button size="xs" variant="ghost" color="red" on:click={() => deleteRequirement(req)} title="Delete">
											Delete
										</Button>
										<Button size="xs" variant="ghost" color="gray" on:click={() => viewHistory(req)} title="View History">
											History
										</Button>
										<Button size="xs" variant="ghost" color="gray" on:click={() => viewReferencedFrom(req)} title="Referenced From">
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