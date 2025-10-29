<script lang="ts">
	import { GitSyncService } from '$lib/gen'
	import Select from './select/Select.svelte'
	import { debounce } from '$lib/utils'

	interface Repository {
		name: string
		url: string
	}

	interface Props {
		disabled?: boolean
		selectedRepository?: string | undefined
		accountId: string
		initialRepositories: Repository[]
		totalCount: number
		perPage: number
		containerClass?: string
		minWidth?: string
		onError?: (error: Error) => void
	}

	let {
		disabled = false,
		selectedRepository = $bindable(),
		accountId,
		initialRepositories,
		totalCount,
		perPage,
		containerClass = 'flex-1',
		minWidth = '160px',
		onError
	}: Props = $props()

	let isFetching = $state(false)
	let selectFilterText = $state('')
	let lastSearchQuery = $state('')

	// Use a derived value that always reflects current repos
	let availableRepos = $derived(() => {
		// If we have search results from backend, use those
		// Otherwise use initial repositories
		return searchResults.length > 0 ? searchResults : initialRepositories
	})

	// Track search results from backend
	let searchResults = $state<Repository[]>([])

	// Only enable search mode if total count exceeds per_page limit
	const searchMode = $derived(totalCount > perPage)

	// Debounced search function
	const debouncedSearch = debounce(async (query: string) => {
		await searchRepositories(query)
	}, 500)

	// Watch for filter text changes and trigger backend search
	$effect(() => {
		if (searchMode && selectFilterText !== undefined && selectFilterText !== lastSearchQuery) {
			if (selectFilterText.length >= 1) {
				// Only search backend if we have more repos than currently loaded
				if (totalCount > initialRepositories.length) {
					debouncedSearch.debounced(selectFilterText)
				}
			} else if (selectFilterText.length === 0) {
				lastSearchQuery = ''
				searchResults = []
			}
		}
	})

	async function searchRepositories(query: string) {
		if (!query) {
			searchResults = []
			lastSearchQuery = ''
			return
		}

		// Don't search if we already have all repos
		if (totalCount <= initialRepositories.length) {
			return
		}

		isFetching = true
		try {
			const installations = await GitSyncService.getGlobalConnectedRepositories({
				search: query
			})

			// Find the matching installation and get its repositories
			const installation = installations.find((inst) => inst.account_id === accountId)
			searchResults = installation?.repositories || []
			lastSearchQuery = query
			isFetching = false
			return searchResults
		} catch (error) {
			isFetching = false
			onError?.(error)
			console.error('Error searching repositories:', error)
			searchResults = []
			lastSearchQuery = ''
			return []
		}
	}
</script>

<div class={containerClass}>
	{#if searchMode}
		<div class="flex flex-col gap-1">
			<Select
				containerStyle={'min-width: ' + minWidth}
				items={availableRepos().map((repo) => ({
					label: repo.name,
					value: repo.url
				}))}
				placeholder={isFetching ? 'Searching...' : `Search all repositories...`}
				clearable
				disabled={disabled || isFetching}
				bind:filterText={selectFilterText}
				bind:value={selectedRepository}
			/>
			{#if totalCount > initialRepositories.length}
				<span class="text-3xs pl-1 text-tertiary">
					Loaded {initialRepositories.length} of {totalCount} repositories.
				</span>
			{/if}
		</div>
	{:else}
		<select bind:value={selectedRepository} {disabled} class="w-full">
			<option value="" disabled selected>Select repository</option>
			{#each initialRepositories as repository (repository.url)}
				<option value={repository.url}>{repository.name}</option>
			{/each}
		</select>
	{/if}
</div>
