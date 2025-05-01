<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { Slack, Code2 } from 'lucide-svelte'
	import MsTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { hubBaseUrlStore, workspaceStore, enterpriseLicense } from '$lib/stores'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/utils'
	import TeamSelector from './TeamSelector.svelte'

	interface TeamItem {
		team_id: string
		team_name: string
	}

	export let platform: 'slack' | 'teams'
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

	let selectedTeam: TeamItem | undefined = undefined

	async function connectTeams() {
		if (!selectedTeam) return

		try {
			await WorkspaceService.connectTeams({
				workspace: $workspaceStore!,
				requestBody: {
					team_id: selectedTeam.team_id,
					team_name: selectedTeam.team_name
				}
			})
			sendUserToast('Connected to Teams successfully')
			onLoadSettings()
		} catch (error) {
			sendUserToast('Failed to connect to Teams', true)
			console.error('Error connecting to Teams:', error)
		}
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
	<div class="flex flex-col gap-2">
		<div class="flex flex-row gap-2 items-center">
			{#if platform === 'teams'}
				<Button
					size="xs"
					color="dark"
					on:click={connectTeams}
					startIcon={{ icon: MsTeamsIcon }}
					disabled={!selectedTeam || !$enterpriseLicense}
				>
					Connect to {platform.charAt(0).toUpperCase() + platform.slice(1)}
					{$enterpriseLicense ? '' : '(EE only)'}
				</Button>

				{#if $enterpriseLicense}
					<TeamSelector
						bind:selectedTeam
						minWidth="180px"
						disabled={!$enterpriseLicense}
						on:error={(e) => sendUserToast('Failed to load teams: ' + e.detail.message, true)}
					/>
				{/if}
			{:else}
				<Button size="xs" color="dark" href={connectHref} startIcon={{ icon: Slack }}>
					Connect to {platform.charAt(0).toUpperCase() + platform.slice(1)}
				</Button>
			{/if}
			<Badge color="red">Not connected</Badge>
		</div>
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
