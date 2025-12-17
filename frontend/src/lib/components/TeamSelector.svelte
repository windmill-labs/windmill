<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { RefreshCcw } from 'lucide-svelte'
	import { WorkspaceService } from '$lib/gen'
	import Select from './select/Select.svelte'
	import { debounce } from '$lib/utils'

	interface TeamItem {
		team_id: string
		team_name: string
	}

	interface Props {
		disabled?: boolean
		selectedTeam?: TeamItem | undefined
		containerClass?: string
		showRefreshButton?: boolean
		teams?: TeamItem[] | undefined
		minWidth?: string
		onError?: (error: Error) => void
		onSelectedTeamChange?: (team: TeamItem | undefined) => void
	}

	let {
		disabled = false,
		selectedTeam = $bindable(),
		containerClass = 'w-64',
		showRefreshButton = true,
		teams = undefined,
		minWidth = '160px',
		onError,
		onSelectedTeamChange
	}: Props = $props()

	let isFetching = $state(false)
	let loadedTeams = $state<TeamItem[]>([])
	let hasLoadedInitial = $state(false)
	let isLoadingMore = $state(false)
	let nextLink = $state<string | null>(null)
	let totalCount = $state(0)

	// Store pre-search state to restore when search is cleared
	let preSearchTeams = $state<TeamItem[] | null>(null)
	let preSearchNextLink = $state<string | null>(null)
	let preSearchTotalCount = $state(0)

	let selectedTeamId = $state<string | undefined>(selectedTeam?.team_id)

	const searchMode = $derived(!teams)

	// Check if there are more teams to load (based on next_link presence)
	const hasMoreTeams = $derived(!!nextLink)
	// Show indicator when we have more teams to load
	const showLoadMoreIndicator = $derived(searchMode && hasMoreTeams)

	let displayTeams = $derived.by(() => {
		const baseTeams = teams || loadedTeams
		if (selectedTeam && !baseTeams.find((t) => t.team_id === selectedTeam?.team_id)) {
			return [selectedTeam, ...baseTeams]
		}
		return baseTeams
	})

	$effect(() => {
		const newTeam = selectedTeamId ? displayTeams.find((t) => t.team_id === selectedTeamId) : undefined

		if (newTeam?.team_id !== selectedTeam?.team_id) {
			selectedTeam = newTeam
		}
	})

	$effect(() => {
		if (selectedTeam?.team_id !== selectedTeamId) {
			selectedTeamId = selectedTeam?.team_id
		}
	})

	let previousTeamId = $state<string | undefined>(undefined)

	$effect(() => {
		if (selectedTeam?.team_id !== previousTeamId) {
			previousTeamId = selectedTeam?.team_id
			onSelectedTeamChange?.(selectedTeam)
		}
	})

	let searchFilterText = $state('')
	let latestSearchQuery = $state('')

	const debouncedSearch = debounce(async (query: string) => {
		latestSearchQuery = query
		await searchTeams(query)
	}, 500)

	// Preload initial teams on mount when in search mode
	$effect(() => {
		if (searchMode && !hasLoadedInitial) {
			hasLoadedInitial = true
			fetchInitialTeams()
		}
	})

	// Track previous search text to detect when cleared
	let previousSearchText = $state('')

	// Handle search input
	$effect(() => {
		if (searchMode) {
			if (searchFilterText.length >= 1) {
				debouncedSearch.debounced(searchFilterText)
				previousSearchText = searchFilterText
			} else if (previousSearchText.length > 0) {
				// Search was cleared - restore pre-search state
				previousSearchText = ''
				restorePreSearchState()
			}
		}
	})

	function restorePreSearchState() {
		latestSearchQuery = ''
		if (preSearchTeams !== null) {
			// Restore the accumulated teams from before the search
			loadedTeams = preSearchTeams
			nextLink = preSearchNextLink
			totalCount = preSearchTotalCount
			// Clear the saved state
			preSearchTeams = null
			preSearchNextLink = null
			preSearchTotalCount = 0
		} else {
			// No saved state, fetch fresh
			fetchInitialTeams()
		}
	}

	async function fetchInitialTeams() {
		isFetching = true
		nextLink = null
		totalCount = 0
		try {
			const response = await WorkspaceService.listAvailableTeamsIds({
				workspace: $workspaceStore!
			})

			loadedTeams =
				response.teams?.map((t) => ({
					team_id: t.team_id || '',
					team_name: t.team_name || ''
				})) || []
			nextLink = response.next_link ?? null
			totalCount = response.total_count ?? loadedTeams.length
		} catch (error) {
			onError?.(error as Error)
			console.error('Error fetching initial teams:', error)
			loadedTeams = []
		} finally {
			isFetching = false
		}
	}

	async function searchTeams(query: string) {
		if (!query) return

		// Save current state before searching (only if not already in search mode)
		if (preSearchTeams === null) {
			preSearchTeams = loadedTeams
			preSearchNextLink = nextLink
			preSearchTotalCount = totalCount
		}

		isFetching = true
		nextLink = null
		try {
			const response = await WorkspaceService.listAvailableTeamsIds({
				workspace: $workspaceStore!,
				search: query
			})

			// Ignore stale results if user typed a different query while waiting
			if (query !== latestSearchQuery) {
				return
			}

			loadedTeams =
				response.teams?.map((t) => ({
					team_id: t.team_id || '',
					team_name: t.team_name || ''
				})) || []
			// Search results don't have pagination
			nextLink = null
		} catch (error) {
			onError?.(error as Error)
			console.error('Error searching teams:', error)
			loadedTeams = []
		} finally {
			isFetching = false
		}
	}

	async function loadMoreTeams() {
		if (isLoadingMore || !nextLink) return

		isLoadingMore = true
		try {
			const response = await WorkspaceService.listAvailableTeamsIds({
				workspace: $workspaceStore!,
				nextLink: nextLink
			})

			// If user started a search while we were loading, discard these results
			// (preSearchTeams was saved before this response, so it would be stale)
			if (preSearchTeams !== null) {
				return
			}

			const newTeams =
				response.teams?.map((t) => ({
					team_id: t.team_id || '',
					team_name: t.team_name || ''
				})) || []

			// Append new teams to existing list
			loadedTeams = [...loadedTeams, ...newTeams]
			nextLink = response.next_link ?? null
		} catch (error) {
			onError?.(error as Error)
			console.error('Error loading more teams:', error)
		} finally {
			isLoadingMore = false
		}
	}

	async function refreshTeams() {
		if (searchMode) {
			if (searchFilterText.length >= 1) {
				latestSearchQuery = searchFilterText
				await searchTeams(searchFilterText)
			} else {
				await fetchInitialTeams()
			}
		}
	}
</script>

<div class={containerClass}>
	<div class="flex flex-col gap-1">
		<div class="flex items-center gap-2">
			<div class="flex-grow" style="min-width: {minWidth};">
				{#if searchMode}
					<Select
						containerStyle={'min-width: ' + minWidth}
						items={displayTeams.map((team) => ({
							label: team.team_name,
							value: team.team_id
						}))}
						placeholder={isFetching ? 'Loading...' : 'Search teams...'}
						clearable
						disabled={disabled || isFetching}
						bind:filterText={searchFilterText}
						bind:value={selectedTeamId}
					/>
				{:else}
					<Select
						containerStyle={'min-width: ' + minWidth}
						items={displayTeams.map((team) => ({
							label: team.team_name,
							value: team.team_id
						}))}
						placeholder="Select a team"
						clearable
						disabled={disabled || isFetching}
						bind:value={selectedTeamId}
					/>
				{/if}
			</div>

			{#if showRefreshButton}
				<button
					onclick={refreshTeams}
					disabled={isFetching || disabled}
					class="flex items-center justify-center p-1.5 rounded hover:bg-surface-hover focus:bg-surface-hover disabled:opacity-50"
					title={searchMode ? 'Refresh teams' : 'Refresh teams from Microsoft'}
				>
					<RefreshCcw size={16} class={isFetching ? 'animate-spin' : ''} />
				</button>
			{/if}
		</div>

		{#if showLoadMoreIndicator}
			<div class="flex items-center gap-2 pl-1">
				<span class="text-2xs text-tertiary">
					Loaded {loadedTeams.length} of {totalCount}
				</span>
				<button
					type="button"
					class="text-2xs text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
					onclick={loadMoreTeams}
					disabled={isLoadingMore}
				>
					{isLoadingMore ? 'loading...' : 'load more...'}
				</button>
			</div>
		{/if}
	</div>
</div>
