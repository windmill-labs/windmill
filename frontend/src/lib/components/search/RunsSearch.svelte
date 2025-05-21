<script lang="ts">
	import { IndexSearchService, type SearchJobsIndexResponse } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { AlertTriangle, Loader2 } from 'lucide-svelte'
	import TimeAgo from '../TimeAgo.svelte'
	import Popover from '../Popover.svelte'
	import { Alert } from '../common'
	import QuickMenuItem from './QuickMenuItem.svelte'
	import { goto } from '$app/navigation'
	import { displayDateOnly } from '$lib/utils'
	import JobPreview from '../runs/JobPreview.svelte'

	let debounceTimeout: any = undefined
	const debouncePeriod: number = 1000

	let loadingCompletedRuns: boolean = $state(false)

	let runSearchRemainingCount: number | undefined = $state(undefined)
	let runSearchTotalCount: number | undefined = $state(undefined)
	let indexMetadata: any = $state({})
	let loadingMoreJobs: boolean = $state(false)

	let {
		mouseMoved = $bindable(),
		selectedWorkspace = $bindable(),
		selectedItem = $bindable(),
		queryParseErrors = $bindable(),
		open = $bindable(),
		loadedRuns = $bindable(),
		selectItem,
		emptySearch
	} = $props<{
		mouseMoved: boolean
		selectedWorkspace: string | undefined
		selectedItem: any
		queryParseErrors: string[]
		open: boolean
		loadedRuns: any[] | undefined
		selectItem: (idx: number) => any
		emptySearch: boolean
	}>()

	export function handleRunSearch(searchTerm: string) {
		clearTimeout(debounceTimeout)
		loadingCompletedRuns = true
		debounceTimeout = setTimeout(async () => {
			clearTimeout(debounceTimeout)
			let searchResults: SearchJobsIndexResponse
			try {
				searchResults = await IndexSearchService.searchJobsIndex({
					searchQuery: searchTerm,
					workspace: $workspaceStore!
				})
				loadedRuns = searchResults.hits
				runSearchTotalCount = searchResults.hit_count
				runSearchRemainingCount = (searchResults.hit_count ?? 0) - loadedRuns?.length
				queryParseErrors = searchResults.query_parse_errors ?? []
				indexMetadata = searchResults.index_metadata
				if (runSearchRemainingCount > 0) {
					loadedRuns.push({ search_id: 'opt:load_more_jobs' })
				}
			} catch (e) {
				sendUserToast(e.body, true)
			}
			loadingCompletedRuns = false
			selectedItem = selectItem(0)
		}, debouncePeriod)
	}
</script>

<div class="flex h-full p-2 divide-x">
	{#if loadingCompletedRuns}
		<div class="flex w-full justify-center items-center h-48">
			<div class="text-tertiary text-center">
				<Loader2 size={34} class="animate-spin" />
			</div>
		</div>
	{:else if loadedRuns && loadedRuns.length > 0}
		<div class="w-4/12 max-h-[70vh] flex flex-col">
			<div class="text-tertiary text-xs">
				{runSearchTotalCount} jobs matched the query
			</div>
			<div class="overflow-y-auto">
				{#each loadedRuns ?? [] as r}
					{#if r.search_id === 'opt:load_more_jobs'}
						<div class="pt-4"></div>
						{#if loadingMoreJobs}
							<div class="pl-8 pb-8 text-tertiary text-center">
								<Loader2 size={20} class="animate-spin" />
							</div>
						{:else}
							<QuickMenuItem
								on:select={() => {
									selectedItem = r
									selectedWorkspace = undefined
									sendUserToast('Loading more jobs', false)
									loadingMoreJobs = true
									setTimeout(() => {
										loadingMoreJobs = false
									}, 2000)
								}}
								id={'opt:load_more_jobs'}
								hovered={selectedItem && r?.search_id === selectedItem?.search_id}
								containerClass="rounded-md px-2 py-1 my-2"
								bind:mouseMoved
							>
								<svelte:fragment slot="itemReplacement">
									<div
										class="py-2 w-full flex flex-row items-center gap-4 transition-all text-secondary text-sm"
									>
										Some other {runSearchRemainingCount} jobs matched the query. Click to load more.
										<!-- Load more ({runSearchRemainingCount} other) -->
										<!-- {runSearchRemainingCount} more documents also matched -->
									</div>
								</svelte:fragment>
							</QuickMenuItem>
						{/if}
					{:else}
						<QuickMenuItem
							on:select={() => {
								selectedItem = r
								selectedWorkspace = r?.document.workspace_id[0]
							}}
							on:keyboardOnlySelect={() => {
								open = false
								goto(`/run/${r?.document.id[0]}`)
							}}
							id={r?.document.id[0]}
							hovered={selectedItem && r?.search_id === selectedItem?.search_id}
							icon={r?.icon}
							containerClass="rounded-md px-2 py-1 my-2"
							bind:mouseMoved
						>
							<svelte:fragment slot="itemReplacement">
								<div class="w-full flex flex-row items-center gap-4 transition-all">
									<div
										class="rounded-full w-2 h-2 {r?.document.success[0]
											? 'bg-green-400'
											: 'bg-red-400'}"
									></div>
									<div class="flex flex-col gap-2">
										<div class="text-xs"> {r?.document.script_path} </div>
										<div class="flex flex-row gap-2">
											<div
												class="whitespace-nowrap col-span-2 !text-tertiary !text-2xs overflow-hidden text-ellipsis flex-shrink text-center"
											>
												{displayDateOnly(new Date(r?.document.created_at[0]))}
											</div>
											<div
												class="whitespace-nowrap col-span-2 !text-tertiary !text-2xs overflow-hidden text-ellipsis flex-shrink text-center"
											>
												<TimeAgo date={r?.document.created_at[0] ?? ''} />
											</div>
										</div>
									</div>
								</div>
							</svelte:fragment>
						</QuickMenuItem>
					{/if}
				{/each}
			</div>
		</div>
		<div class="w-8/12 max-h-[70vh]">
			{#if selectedItem === undefined}
				Select a result to preview
			{:else}
				<div class="h-[95%] overflow-y-scroll">
					<JobPreview id={selectedItem?.document?.id[0]} workspace={selectedWorkspace} />
				</div>
			{/if}
			<div class="flex flex-row pt-3 pl-4 items-center text-xs text-secondary">
				{#if indexMetadata.indexed_until}
					<span class="px-2">
						Most recently indexed job was created at <TimeAgo
							agoOnlyIfRecent
							date={indexMetadata.indexed_until || ''}
						/>
					</span>
				{/if}
				{#if indexMetadata.lost_lock_ownership}
					<Popover notClickable placement="top">
						<AlertTriangle size={16} class="text-gray-500" />
						<svelte:fragment slot="text">
							The current indexer is no longer indexing new jobs. This is most likely because of an
							ongoing deployment and indexing will resume once it's complete.
						</svelte:fragment>
					</Popover>
				{/if}
			</div>
		</div>
	{:else}
		<div class="flex flex-col h-full w-full justify-center items-center h-48">
			<div class="text-tertiary text-center">
				{#if emptySearch}
					<div class="text-2xl font-bold">Enter your search terms</div>
					<div class="text-sm">Start typing to do full-text search across completed runs</div>
				{:else}
					<div class="text-2xl font-bold">No runs found</div>
					<div class="text-sm">There were no completed runs that match your query</div>
				{/if}
				<div class="text-sm">
					Note that new runs might take a while to become searchable (by default ~5min)
				</div>
				{#if !$enterpriseLicense}
					<div class="py-6"></div>

					<Alert title="This is an EE feature" type="warning">
						Full-text search on jobs is only available on EE.
					</Alert>
				{/if}
			</div>
			<div class="flex flex-row pt-10 text-xs text-secondary">
				{#if indexMetadata.indexed_until}
					<span class="px-2">
						Most recently indexed job was created at <TimeAgo
							agoOnlyIfRecent
							date={indexMetadata.indexed_until}
						/>
					</span>
				{/if}
				{#if indexMetadata.lost_lock_ownership}
					<Popover notClickable placement="top">
						<AlertTriangle size={16} class="text-gray-500" />
						<svelte:fragment slot="text">
							The current indexer is no longer indexing new jobs. This is most likely because of an
							ongoing deployment and indexing will resume once it's complete.
						</svelte:fragment>
					</Popover>
				{/if}
			</div>
		</div>
	{/if}
</div>
