<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Slack, Code2, Unplug, Plug } from 'lucide-svelte'
	import MsTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { hubBaseUrlStore, workspaceStore, enterpriseLicense } from '$lib/stores'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/utils'
	import TeamSelector from './TeamSelector.svelte'
	import CollapseLink from './CollapseLink.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	interface TeamItem {
		team_id: string
		team_name: string
	}

	let {
		platform,
		teamName,
		display_name,
		scriptPath = $bindable(),
		initialPath = $bindable(),
		itemKind = $bindable('script' as 'flow' | 'script'),
		onDisconnect,
		onSelect,
		connectHref,
		createScriptHref,
		createFlowHref,
		documentationLink,
		onLoadSettings,
		workspaceConfig,
		hideConnectButton = false,
		isOAuthEnabled = false,
		workspaceSpecificConnection = false
	}: {
		platform: 'slack' | 'teams'
		teamName: string | undefined
		display_name: string | undefined
		scriptPath: string
		initialPath: string
		itemKind: 'flow' | 'script'
		onDisconnect: () => Promise<void>
		onSelect: () => Promise<void>
		connectHref: string | undefined
		createScriptHref: string
		createFlowHref: string
		documentationLink: string
		onLoadSettings: () => void
		workspaceConfig?: import('svelte').Snippet
		hideConnectButton?: boolean
		isOAuthEnabled?: boolean
		workspaceSpecificConnection?: boolean
	} = $props()

	let selectedTeam: TeamItem | undefined = $state(undefined)

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
			// Extract the actual error message from the API response
			let errorMessage = 'Failed to connect to Teams'

			if (typeof error?.body === 'string') {
				errorMessage = error.body
			} else if (error?.body?.message) {
				errorMessage = error.body.message
			} else if (error?.message && error.message !== 'Bad Request') {
				errorMessage = error.message
			}

			sendUserToast(errorMessage, true)
			console.error('Error connecting to Teams:', error)
		}
	}

	const capitalizedPlatform = $derived(platform.charAt(0).toUpperCase() + platform.slice(1))
</script>

<SettingCard label={capitalizedPlatform + ' connection'}>
	{#if workspaceConfig}
		{@render workspaceConfig()}
	{/if}
	{#if teamName || workspaceSpecificConnection}
		<div class="flex flex-col gap-2 max-w-sm">
			<div class="flex flex-row gap-2 items-center">
				{#if display_name}
					<Badge color="green">
						<Plug size={14} />
						Workspace connected to {capitalizedPlatform} team '{display_name}'</Badge
					>
				{/if}
				<Button
					unifiedSize="md"
					startIcon={{ icon: Unplug }}
					disabled={!$enterpriseLicense && platform === 'teams'}
					onclick={onDisconnect}
					destructive
					variant="subtle"
				>
					Disconnect {capitalizedPlatform}
					{!$enterpriseLicense && platform === 'teams' ? '(EE only)' : ''}
				</Button>
			</div>
		</div>
	{:else if !hideConnectButton}
		<div class="flex flex-col gap-2">
			<div class="flex flex-row gap-2 items-center">
				{#if platform === 'teams'}
					{#if $enterpriseLicense && isOAuthEnabled}
						<TeamSelector
							bind:selectedTeam
							minWidth="180px"
							disabled={!$enterpriseLicense}
							onError={(e) => {
								const errorMsg =
									typeof (e as any)?.body === 'string'
										? (e as any).body
										: e?.message || 'Unknown error'
								sendUserToast('Failed to load teams: ' + errorMsg, true)
							}}
						/>
					{/if}
					<Button
						unifiedSize="md"
						variant="accent"
						onclick={connectTeams}
						endIcon={{ icon: MsTeamsIcon }}
						disabled={!selectedTeam || !$enterpriseLicense}
					>
						Connect to {platform.charAt(0).toUpperCase() + platform.slice(1)}
						{$enterpriseLicense ? '' : '(EE only)'}
					</Button>
				{:else}
					<Button
						size="xs"
						variant="accent"
						href={connectHref}
						startIcon={{ icon: Slack }}
						disabled={!isOAuthEnabled}
					>
						Connect to {platform.charAt(0).toUpperCase() + platform.slice(1)}
					</Button>
				{/if}
			</div>
		</div>
	{/if}
</SettingCard>

<SettingCard
	label="Script or flow to run on /windmill command"
	description="Pick a script or flow meant to be triggered when the `/windmill` command is invoked."
>
	<div class="flex flex-row gap-2">
		<ScriptPicker
			kinds={['script']}
			allowFlow
			bind:itemKind
			bind:scriptPath
			{initialPath}
			on:select={onSelect}
			disabled={!teamName || (!$enterpriseLicense && platform === 'teams')}
			clearable
		/>

		{#if teamName && ($enterpriseLicense || platform === 'slack') && (scriptPath === '' || scriptPath === undefined)}
			{#if itemKind === 'script'}
				<Button size="sm" endIcon={{ icon: Code2 }} href={createScriptHref}>
					Create a script from template to handle {platform} commands
				</Button>
			{:else if itemKind === 'flow'}
				<Button size="sm" endIcon={{ icon: BarsStaggered }} href={createFlowHref}>
					Create a flow from template to handle {platform} commands
				</Button>
			{/if}
		{/if}
	</div>

	{#if !teamName}
		<div class="text-red-500 text-xs"
			>Please connect your workspace to {capitalizedPlatform} to use this feature</div
		>
	{/if}

	<CollapseLink text="How to use">
		<div class="prose text-2xs text-primary">
			Upon connection, templates for a <a href="{$hubBaseUrlStore}/scripts/{platform}/1405/"
				>script</a
			>
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
	</CollapseLink>
</SettingCard>
