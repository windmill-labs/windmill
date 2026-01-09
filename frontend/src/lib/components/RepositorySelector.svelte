<script lang="ts">
	import { GitSyncService } from '$lib/gen'
	import Select from './select/Select.svelte'

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

	// Track all loaded repositories across pages
	let loadedRepositories = $state<Repository[]>(initialRepositories)
	let currentPage = $state(1)
	let isLoadingMore = $state(false)

	// Client-side filtered repositories based on search text
	let filterText = $state('')

	const filteredRepositories = $derived(() => {
		if (!filterText) {
			return loadedRepositories
		}
		const search = filterText.toLowerCase()
		return loadedRepositories.filter((repo) => repo.name.toLowerCase().includes(search))
	})

	// Check if there are more repos to load
	const hasMoreRepos = $derived(loadedRepositories.length < totalCount)

	// Show the select when we have more repos than the initial load
	const showSearchableSelect = $derived(totalCount > perPage)

	async function loadMoreRepositories() {
		if (isLoadingMore || !hasMoreRepos) {
			return
		}

		isLoadingMore = true
		const nextPage = currentPage + 1

		try {
			const installations = await GitSyncService.getGlobalConnectedRepositories({
				page: nextPage
			})

			// Find the matching installation and get its repositories
			const installation = installations.find((inst) => inst.account_id === accountId)

			if (installation?.repositories) {
				// Append new repos to existing ones
				loadedRepositories = [...loadedRepositories, ...installation.repositories]
				currentPage = nextPage
			}
		} catch (error) {
			onError?.(error as Error)
			console.error('Error loading more repositories:', error)
		} finally {
			isLoadingMore = false
		}
	}
</script>

<div class={containerClass}>
	{#if showSearchableSelect}
		<div class="flex flex-col gap-1">
			<Select
				containerStyle={'min-width: ' + minWidth}
				items={filteredRepositories().map((repo) => ({
					label: repo.name,
					value: repo.url
				}))}
				placeholder="Select repository..."
				clearable
				disabled={disabled}
				bind:filterText={filterText}
				bind:value={selectedRepository}
			/>
			{#if hasMoreRepos}
				<div class="flex items-center gap-2 pl-1">
					<span class="text-2xs text-tertiary">
						Loaded {loadedRepositories.length} of {totalCount}
					</span>
					{#if hasMoreRepos}
						<button
							type="button"
							class="text-2xs text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
							onclick={loadMoreRepositories}
							disabled={isLoadingMore}
						>
							{isLoadingMore ? 'loading...' : 'load more...'}
						</button>
					{/if}
				</div>
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
