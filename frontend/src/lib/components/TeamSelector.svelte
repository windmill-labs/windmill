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
	let searchResults = $state<TeamItem[]>([])

	let selectedTeamId = $state<string | undefined>(selectedTeam?.team_id)

	const searchMode = !teams

	let displayTeams = $derived.by(() => {
		const baseTeams = teams || searchResults
		if (selectedTeam && !baseTeams.find(t => t.team_id === selectedTeam?.team_id)) {
			return [selectedTeam, ...baseTeams]
		}
		return baseTeams
	})

	$effect(() => {
		const newTeam = selectedTeamId
			? displayTeams.find(t => t.team_id === selectedTeamId)
			: undefined

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

	const debouncedSearch = debounce(async (query: string) => {
		await searchTeams(query)
	}, 500)

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
					items={displayTeams.map((team) => ({
						label: team.team_name,
						value: team.team_id
					}))}
					placeholder={isFetching ? "Searching..." : "Search teams..."}
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
