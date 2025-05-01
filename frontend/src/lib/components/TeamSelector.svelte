<script lang="ts">
	import Select from '$lib/components/apps/svelte-select/lib/Select.svelte'
	import { workspaceStore } from '$lib/stores'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import { createEventDispatcher, onMount } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { RefreshCcw } from 'lucide-svelte'
	import { WorkspaceService, TeamsService } from '$lib/gen'

	interface TeamItem {
		team_id: string;
		team_name: string;
	}

	export let disabled = false
	export let placeholder = 'Select a team'
	export let selectedTeam: TeamItem | undefined = undefined
	export let containerClass = 'w-64'
	export let showRefreshButton = true
	export let teams: TeamItem[] | undefined = undefined
	export let enableSync = false
	export let minWidth = '160px'

	let isFetching = false
	let isSyncing = false
	let darkMode: boolean = false

	const dispatch = createEventDispatcher<{
		change: TeamItem
		error: Error
		synced: TeamItem[]
	}>()

	function onThemeChange() {
		darkMode = document.documentElement.classList.contains('dark')
	}

	onMount(() => {
		onThemeChange()
		if (!teams) {
			loadTeams()
		}
	})

	async function syncTeams() {
		if (isSyncing) return
		isSyncing = true
		isFetching = true

		try {
			const syncedTeams = await TeamsService.syncTeams()
			if (teams) {
				teams = syncedTeams;
			}
			dispatch('synced', syncedTeams)
			isSyncing = false
			isFetching = false
			return syncedTeams
		} catch (error) {
			dispatch('error', error)
			console.error('Error syncing teams:', error)
			isSyncing = false
			isFetching = false
			return null
		}
	}

	async function loadTeams() {
		if (teams) return

		isFetching = true
		try {
			const response = await WorkspaceService.listAvailableTeamsIds({
				workspace: $workspaceStore!
			}) as unknown as TeamItem[]

			teams = response || []
			isFetching = false
			console.log('Teams loaded:', teams.length)
			return teams
		} catch (error) {
			isFetching = false
			dispatch('error', error)
			console.error('Error loading teams:', error)
			return []
		}
	}

	async function refreshTeams() {
		isFetching = true;
		try {
			if (enableSync) {
				const syncedTeams = await syncTeams();
				if (syncedTeams) {
					if (teams) {
						teams = syncedTeams;
						isFetching = false;
						return syncedTeams;
					}
				}
			}

			if (!teams) {
				return await loadTeams();
			} else {
				isFetching = false;
			}
		} catch (error) {
			isFetching = false;
			dispatch('error', error);
		}
	}

	function handleTeamSelect(event) {
		selectedTeam = event.detail
		if (selectedTeam) {
			dispatch('change', selectedTeam)
		}
	}

	function filterTeams(filterText: string | unknown) {
		if (!teams) return teams

		const searchText = typeof filterText === 'string' ? filterText : '';

		const filtered = searchText
			? teams.filter(team =>
				team.team_name.toLowerCase().includes(searchText.toLowerCase()))
			: teams

		return filtered
	}
</script>

<div class={containerClass}>
	<div class="flex items-center gap-2">
		<div class="flex-grow" style="min-width: {minWidth};">
			<Select
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={'border-color: lightgray; min-width: ' + minWidth + ';' +
					(darkMode
						? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
						: SELECT_INPUT_DEFAULT_STYLE.containerStyles)}
				itemId="team_id"
				label="team_name"
				items={teams || []}
				on:change={handleTeamSelect}
				{placeholder}
				searchable={true}
				loading={isFetching}
				disabled={disabled || isFetching}
				on:input={async (e) => {
					const filterText = e.detail
					if (!teams) {
						await loadTeams()
					}
					return filterTeams(filterText)
				}}
			/>
		</div>

		{#if showRefreshButton}
			<button
				on:click={refreshTeams}
				disabled={isFetching || disabled}
				class="flex items-center justify-center p-1.5 rounded hover:bg-surface-hover focus:bg-surface-hover disabled:opacity-50"
				title={enableSync ? "Sync and refresh teams list" : "Refresh teams list"}
			>
				<RefreshCcw size={16} class={isFetching ? 'animate-spin' : ''} />
			</button>
		{/if}
	</div>

	{#if isFetching || ((!teams || teams.length === 0) && !isFetching)}
		<div class="text-xs text-tertiary mt-1">
			{#if isFetching}
				{isSyncing ? 'Syncing and loading teams...' : 'Loading teams...'}
			{:else}
				No available teams found
			{/if}
		</div>
	{/if}
</div>

<DarkModeObserver on:change={onThemeChange} />