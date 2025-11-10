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
	import { Plus, FileText, Folder, Search, Code2, Edit, Eye } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import type uFuzzy from '@leeoniya/ufuzzy'

	// Mock data structure for requirements - replace with actual API calls later
	interface Requirement {
		path: string
		content?: string
		description?: string
		language?: string
		documentation?: string
		tags?: string[]
		created_at: string
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

	let owners = $derived(
		Array.from(
			new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
		).sort()
	)

	let ownerFilter: string | undefined = $state(undefined)

	$effect(() => {
		if ($workspaceStore) {
			ownerFilter = undefined
		}
	})

	let preFilteredItems = $derived(
		ownerFilter == undefined
			? requirements
			: requirements?.filter((x) => x.path.startsWith(ownerFilter ?? ''))
	)

	// Mock function to load requirements - replace with actual API call
	async function loadRequirements(): Promise<void> {
		// Simulate API call with mock data showing requirements can exist anywhere
		requirements = [
			{
				path: '/my_global_requirement',
				content: 'requests>=2.31.0\npandas>=2.0.0\nnumpy>=1.24.0',
				description: 'Global Python requirements for workspace-wide policies',
				language: 'python',
				documentation: 'This applies to all Python scripts in the workspace.',
				tags: ['global', 'policy'],
				created_at: new Date().toISOString(),
				canWrite: true
			},
			{
				path: '/u/req_for_all_users',
				content: '{\n  "dependencies": {\n    "axios": "^1.6.0",\n    "lodash": "^4.17.21"\n  }\n}',
				description: 'TypeScript requirements that apply to all users',
				language: 'typescript',
				documentation: 'User-specific TypeScript requirements and guidelines.',
				tags: ['users', 'guidelines'],
				created_at: new Date().toISOString(),
				canWrite: true
			},
			{
				path: '/u/admin/req_for_all_scripts_and_flows_for_admin',
				content: 'module windmill-admin-script\n\ngo 1.21\n\nrequire (\n\tgithub.com/gorilla/mux v1.8.1\n)',
				description: 'Go requirements for admin scripts and flows',
				language: 'go',
				documentation: 'Go-specific requirements for admin operations.',
				tags: ['admin', 'scripts', 'flows'],
				created_at: new Date().toISOString(),
				canWrite: true
			},
			{
				path: '/u/admin/my_script/req_for_my_script',
				content: '{\n  "require": {\n    "guzzlehttp/guzzle": "^7.8",\n    "monolog/monolog": "^3.5"\n  }\n}',
				description: 'Specific PHP requirements for my_script',
				language: 'php',
				documentation: 'PHP-specific error handling requirements.',
				tags: ['script-specific', 'error-handling'],
				created_at: new Date().toISOString(),
				canWrite: true
			}
		]
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

	function editRequirement(path: string) {
		requirementEditor?.editRequirement(path)
	}

	function viewRequirement(path: string, content: string, language: string) {
		viewPath = path
		viewContent = content || ''
		viewLanguage = language || 'python'
		viewDrawer?.openDrawer()
	}

	function getTagsFromPath(path: string): string[] {
		const segments = path.split('/').filter(Boolean)
		return segments.slice(0, -1) // All segments except the last one (filename)
	}

	function truncate(str: string, length: number): string {
		return str.length > length ? str.substring(0, length) + '...' : str
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
	f={(x) => x.path + ' ' + (x.description || '') + ' ' + (x.content || '')}
	{opts}
/>

<CenteredPage>
	<PageHeader
		title="Requirements"
		tooltip="Requirements can be stored anywhere in the workspace and define rules, policies, or specifications for scripts, flows, and other resources."
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
				placeholder="Search requirements by path, content, or tags..."
				bind:value={filter}
				class="bg-surface !h-10 !px-4 !pr-10 !rounded-lg text-sm focus:outline-none w-full"
			/>
			<button aria-label="Search" type="submit" class="absolute right-0 top-0 mt-3 mr-4">
				<Search class="h-4 w-4" />
			</button>
		</div>
	</div>

	<div class="min-h-[56px]">
		<ListFilters bind:selectedFilter={ownerFilter} filters={owners} />
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
						<Cell head first>Path</Cell>
						<Cell head>Language</Cell>
						<Cell head>Description</Cell>
						<Cell head>Tags</Cell>
						<Cell head last>Actions</Cell>
					</tr>
				</Head>
				<tbody class="divide-y">
					{#each filteredItems as { path, description, content, language, canWrite, marked }}
						{@const pathTags = getTagsFromPath(path)}
						<Row>
							<Cell first>
								<div class="flex items-center gap-2">
									<FileText size={16} class="text-secondary" />
									<a
										class="break-all hover:text-primary cursor-pointer"
										onclick={() => editRequirement(path)}
										title={path}
									>
										{#if marked}
											{@html marked}
										{:else}
											{path}
										{/if}
									</a>
								</div>
							</Cell>
							<Cell>
								<div class="flex items-center gap-1">
									<Code2 size={14} class="text-secondary" />
									<span class="text-xs font-mono text-secondary">
										{language || 'python'}
									</span>
								</div>
							</Cell>
							<Cell>
								<span class="text-xs text-tertiary">
									{truncate(description ?? '', 60)}
								</span>
							</Cell>
							<Cell>
								<div class="flex flex-wrap gap-1">
									{#each pathTags as tag}
										<span
											class="inline-flex items-center gap-x-1 rounded-md bg-surface-secondary px-2 py-1 text-2xs font-medium text-secondary"
										>
											<Folder size={10} />
											{tag}
										</span>
									{/each}
								</div>
							</Cell>
							<Cell last>
								<div class="flex gap-2">
									<Button size="xs" variant="border" color="light" startIcon={{ icon: Eye }} on:click={() => viewRequirement(path, content || '', language || 'python')}>
										View
									</Button>
									{#if canWrite}
										<Button size="xs" variant="border" color="light" startIcon={{ icon: Edit }} on:click={() => editRequirement(path)}>
											Edit
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