<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { RefreshCcw } from 'lucide-svelte'
	import { WorkspaceService } from '$lib/gen'
	import Select from './Select.svelte'
	import { onMount } from 'svelte'

	interface TeamItem {
		team_id: string
		team_name: string
	}

	interface Props {
		disabled?: boolean
		placeholder?: string
		selectedTeam?: TeamItem | undefined
		containerClass?: string
		showRefreshButton?: boolean
		teams?: TeamItem[] | undefined
		minWidth?: string
		onError?: (error: Error) => void
	}

	let {
		disabled = false,
		placeholder = 'Select a team',
		selectedTeam = $bindable(),
		containerClass = 'w-64',
		showRefreshButton = true,
		teams = undefined,
		minWidth = '160px',
		onError
	}: Props = $props()

	let isFetching = $state(false)
	onMount(() => {
		if (!teams) loadTeams()
	})

	async function loadTeams() {
		isFetching = true
		try {
			const response = (await WorkspaceService.listAvailableTeamsIds({
				workspace: $workspaceStore!
			})) as unknown as TeamItem[]

			teams = response || []
			isFetching = false
			console.log('Teams loaded:', teams.length)
			return teams
		} catch (error) {
			isFetching = false
			onError?.(error)
			console.error('Error loading teams:', error)
			return []
		}
	}
</script>

<div class={containerClass}>
	<div class="flex items-center gap-2">
		<div class="flex-grow" style="min-width: {minWidth};">
			<Select
				containerStyle={'min-width: ' + minWidth}
				items={teams?.map((team) => ({
					label: team.team_name,
					value: team.team_id
				})) ?? []}
				{placeholder}
				clearable
				disabled={disabled || isFetching}
				bind:value={
					() => selectedTeam?.team_id,
					(value) => (selectedTeam = teams?.find((team) => team.team_id === value))
				}
			/>
		</div>

		{#if showRefreshButton}
			<button
				onclick={loadTeams}
				disabled={isFetching || disabled}
				class="flex items-center justify-center p-1.5 rounded hover:bg-surface-hover focus:bg-surface-hover disabled:opacity-50"
				title="Refresh teams from Microsoft"
			>
				<RefreshCcw size={16} class={isFetching ? 'animate-spin' : ''} />
			</button>
		{/if}
	</div>

	{#if isFetching || ((!teams || teams.length === 0) && !isFetching)}
		<div class="text-xs text-tertiary mt-1">
			{#if isFetching}
				Fetching teams from Microsoft...
			{:else}
				No available teams found
			{/if}
		</div>
	{/if}
</div>
