<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { Slack, Code2, RefreshCcw } from 'lucide-svelte'
	import MsTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { hubBaseUrlStore, workspaceStore, enterpriseLicense } from '$lib/stores'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { WorkspaceService } from '$lib/gen'
	import type { ListAvailableTeamsIdsResponse } from '$lib/gen/types.gen'
	export let platform: 'slack' | 'teams'
	import { sendUserToast } from '$lib/utils'
	export let teamName: string | undefined
	export let display_name: string | undefined
	export let scriptPath: string
	export let initialPath: string
	export let onDisconnect: () => Promise<void>
	export let onSelect: () => Promise<void>
	export let connectHref: string | undefined
	export let createScriptHref: string
	export let createFlowHref: string
	export let documentationLink: string
	export let onLoadSettings: () => void
	export let itemKind: 'flow' | 'script' = 'script'

	let isFetching = false

	let teams: ListAvailableTeamsIdsResponse = []
	let selected_teams_team: string | undefined = undefined

	async function loadTeams() {
		isFetching = true
		selected_teams_team = undefined
		teams = (await WorkspaceService.listAvailableTeamsIds({ workspace: $workspaceStore! })) ?? []
		isFetching = false
	}

	$: workspaceStore && platform && $enterpriseLicense && loadTeams()

	async function connectTeams() {
		const selectedTeam = teams.find((team) => team.team_id === selected_teams_team)
		const selectedTeamName = selectedTeam ? selectedTeam.team_name : undefined

		await WorkspaceService.connectTeams({
			workspace: $workspaceStore!,
			requestBody: {
				team_id: selected_teams_team,
				team_name: selectedTeamName
			}
		})
		sendUserToast('Connected to Teams to Workspace')
		onLoadSettings()
		loadTeams()
	}
</script>

<div class="flex flex-col gap-1">
	<div class="text-primary font-semibold"
		>Connect Workspace to {platform.charAt(0).toUpperCase() + platform.slice(1)}</div
	>
	<Description link={documentationLink}>
		Connect your Windmill workspace to your {platform} workspace to trigger a script or a flow with a
		'/windmill' command.
	</Description>
</div>

{#if teamName}
	<div class="flex flex-col gap-2 max-w-sm">
		<div class="flex flex-row gap-2">
			<Button
				size="sm"
				endIcon={{ icon: platform === 'slack' ? Slack : MsTeamsIcon }}
				btnClasses="mt-2"
				variant="border"
				disabled={!$enterpriseLicense && platform === 'teams'}
				on:click={onDisconnect}
			>
				Disconnect {platform.charAt(0).toUpperCase() + platform.slice(1)}
				{!$enterpriseLicense && platform === 'teams' ? '(EE only)' : ''}
			</Button>
			{#if display_name}
				<Badge class="mt-2" color="green">Connected to Team '{display_name}'</Badge>
			{/if}
		</div>
		{#if $enterpriseLicense || platform === 'slack'}
			<Button size="sm" endIcon={{ icon: Code2 }} href={createScriptHref}>
				Create a script to handle {platform} commands
			</Button>
			<Button size="sm" endIcon={{ icon: BarsStaggered }} href={createFlowHref}>
				Create a flow to handle {platform} commands
			</Button>
		{/if}
	</div>
{:else}
	<div class="flex flex-row gap-2">
		{#if platform === 'teams'}
			<Button
				size="xs"
				color="dark"
				on:click={connectTeams}
				startIcon={{ icon: MsTeamsIcon }}
				disabled={!selected_teams_team || !enterpriseLicense}
			>
				Connect to {platform.charAt(0).toUpperCase() + platform.slice(1)}
				{$enterpriseLicense ? '' : '(EE only)'}
			</Button>
			{#if $enterpriseLicense}
				<div class="w-64 flex flex-row gap-2">
					<select bind:value={selected_teams_team}>
						{#if !isFetching}
							{#if teams.length === 0}
								<option value="" disabled selected>No unassigned teams found</option>
							{:else}
								<option value="" disabled selected>Select team</option>
								{#each teams as team}
									<option value={team.team_id}>
										{team.team_name}
									</option>
								{/each}
							{/if}
						{:else}
							<option value="" disabled selected>Loading...</option>
						{/if}
					</select>
				</div>
				<div class="pt-1">
					<button on:click={loadTeams} class="flex items-center gap-1 mt-2">
						<RefreshCcw size={16} class={isFetching ? 'animate-spin' : ''} />
					</button>
				</div>
			{/if}
		{:else}
			<Button size="xs" color="dark" href={connectHref} startIcon={{ icon: Slack }}>
				Connect to {platform.charAt(0).toUpperCase() + platform.slice(1)}
			</Button>
		{/if}
		<Badge color="red">Not connected</Badge>
	</div>
{/if}

<div class="bg-surface-disabled p-4 rounded-md flex flex-col gap-1">
	<div class="text-primary font-md font-semibold"> Script or flow to run on /windmill command </div>
	<div class="relative">
		{#if !teamName || (!$enterpriseLicense && platform === 'teams')}
			<div class="absolute top-0 right-0 bottom-0 left-0 bg-surface-disabled/50 z-40"></div>
		{/if}
		<ScriptPicker
			kinds={['script']}
			allowFlow
			bind:itemKind
			bind:scriptPath
			{initialPath}
			on:select={onSelect}
		/>
	</div>

	<div class="prose text-2xs text-tertiary">
		Pick a script or flow meant to be triggered when the `/windmill` command is invoked. Upon
		connection, templates for a <a href="{$hubBaseUrlStore}/scripts/{platform}/1405/">script</a>
		and <a href="{$hubBaseUrlStore}/flows/28/">flow</a> are available.

		<br /><br />

		The script or flow chosen is passed the parameters `response_url: string` and `text: string`
		respectively the url to reply directly to the trigger and the text of the command.

		<br /><br />

		It can take additionally the following args: channel_id, user_name, user_id, command,
		trigger_id, api_app_id

		<br /><br />

		<span class="font-bold text-xs">
			The script or flow is permissioned as group "{platform}" that will be automatically created
			after connection to {platform.charAt(0).toUpperCase() + platform.slice(1)}.
		</span>

		<br /><br />

		See more on
		<a href={documentationLink}>documentation</a>.
	</div>
</div>
