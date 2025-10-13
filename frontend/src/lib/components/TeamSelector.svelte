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
	}

	let {
		disabled = false,
		selectedTeam = $bindable(),
		containerClass = 'w-64',
		showRefreshButton = true,
		teams = undefined,
		minWidth = '160px',
		onError
	}: Props = $props()

	let isFetching = $state(false)
	let searchResults = $state<TeamItem[]>([])

	// Only enable search mode if no teams are provided
	const searchMode = !teams

	// Determine which teams to show: provided teams or search results
	// In search mode, include the selected team if it exists
	let displayTeams = $derived(() => {
		const baseTeams = teams || searchResults;
		if (searchMode && selectedTeam && !baseTeams.find(t => t.team_id === selectedTeam?.team_id)) {
			return [selectedTeam, ...baseTeams];
		}
		return baseTeams;
	})

	// Create separate filter text for search mode
	let searchFilterText = $state('')

	// Debounced search function
	const debouncedSearch = debounce(async (query: string) => {
		await searchTeams(query)
	}, 500)

	// Watch for search filter text changes (only in search mode)
	$effect(() => {
		if (searchMode) {
			if (searchFilterText.length >= 1) {
				debouncedSearch.debounced(searchFilterText)
			} else if (searchFilterText.length === 0) {
				searchResults = []
			}
		}
	})

	async function searchTeams(query: string) {
		if (!query) return

		isFetching = true
		try {
			const response = (await WorkspaceService.listAvailableTeamsIds({
				workspace: $workspaceStore!,
				search: query
			})) as unknown as TeamItem[]

			searchResults = response || []
			isFetching = false
			return searchResults
		} catch (error) {
			isFetching = false
			onError?.(error)
			console.error('Error searching teams:', error)
			searchResults = []
			return []
		}
	}

	async function refreshSearch() {
		if (searchMode && searchFilterText.length >= 2) {
			await searchTeams(searchFilterText)
		}
	}
</script>

<div class={containerClass}>
	<div class="flex items-center gap-2">
		<div class="flex-grow" style="min-width: {minWidth};">
			{#if searchMode}
				<Select
					containerStyle={'min-width: ' + minWidth}
					items={searchFilterText.length >= 1 || (searchFilterText.length === 0 && selectedTeam) ? displayTeams().map((team) => ({
						label: team.team_name,
						value: team.team_id
					})) : []}
					placeholder={isFetching ? "Searching..." : "Search teams..."}
					clearable
					disabled={disabled || isFetching}
					bind:filterText={searchFilterText}
					bind:value={
						() => selectedTeam?.team_id,
						(value) => {
							selectedTeam = value ? displayTeams().find((team) => team.team_id === value) : undefined
						}
					}
				/>
			{:else}
				<Select
					containerStyle={'min-width: ' + minWidth}
					items={displayTeams().map((team) => ({
						label: team.team_name,
						value: team.team_id
					}))}
					placeholder="Select a team"
					clearable
					disabled={disabled || isFetching}
					bind:value={
						() => selectedTeam?.team_id,
						(value) => {
							selectedTeam = value ? displayTeams().find((team) => team.team_id === value) : undefined
						}
					}
				/>
			{/if}
		</div>

		{#if showRefreshButton}
			<button
				onclick={refreshSearch}
				disabled={isFetching || disabled || (searchMode && searchFilterText.length < 2)}
				class="flex items-center justify-center p-1.5 rounded hover:bg-surface-hover focus:bg-surface-hover disabled:opacity-50"
				title={searchMode ? "Refresh search results" : "Refresh teams from Microsoft"}
			>
				<RefreshCcw size={16} class={isFetching ? 'animate-spin' : ''} />
			</button>
		{/if}
	</div>

</div>
