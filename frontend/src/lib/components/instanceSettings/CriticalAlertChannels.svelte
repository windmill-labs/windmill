<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import SlackChannelCard from './SlackChannelCard.svelte'
	import TeamsChannelCard from './TeamsChannelCard.svelte'
	import EmailChannelCard from './EmailChannelCard.svelte'
	import EEOnly from '../EEOnly.svelte'
	import type { Writable } from 'svelte/store'
	import { isSmtpSettingsValid } from './SmtpSettings.svelte'

	interface Props {
		values: Writable<Record<string, any>>
		openSmtpSettings?: () => void
		oauths?: Record<string, any>
	}

	let { values, openSmtpSettings, oauths }: Props = $props()

	// Derived state for each channel type
	const slackChannels = $derived.by(() => {
		const channels = $values?.critical_error_channels || []
		return channels.filter((channel: any) => channel && 'slack_channel' in channel)
	})

	const teamsChannels = $derived.by(() => {
		const channels = $values?.critical_error_channels || []
		return channels.filter((channel: any) => channel && 'teams_channel' in channel)
	})

	const emailChannels = $derived.by(() => {
		const channels = $values?.critical_error_channels || []
		return channels.filter((channel: any) => channel && 'email' in channel)
	})

	const slackTeamName = $derived($values['slack']?.['team_name'])

	// Teams OAuth validation function
	function isTeamsOAuthConfigured(teamsConfig: any): boolean {
		return (
			teamsConfig &&
			teamsConfig.id?.trim() &&
			teamsConfig.secret?.trim() &&
			teamsConfig.tenant?.trim()
		)
	}

	const isTeamsConnected = $derived(isTeamsOAuthConfigured(oauths?.teams))

	// Compute dynamic order based on channel presence
	const slackOrder = $derived(slackChannels.length > 0 ? 1 : 4)
	const teamsOrder = $derived(teamsChannels.length > 0 ? 2 : 5)
	const emailOrder = $derived(emailChannels.length > 0 ? 3 : 6)

	function addSlackChannel() {
		if (
			$values.critical_error_channels == undefined ||
			!Array.isArray($values.critical_error_channels)
		) {
			$values.critical_error_channels = []
		}
		$values.critical_error_channels = $values.critical_error_channels.concat({ slack_channel: '' })
	}

	function addTeamsChannel() {
		if (
			$values.critical_error_channels == undefined ||
			!Array.isArray($values.critical_error_channels)
		) {
			$values.critical_error_channels = []
		}
		$values.critical_error_channels = $values.critical_error_channels.concat({
			teams_channel: undefined
		})
	}

	function addEmailChannel() {
		if (
			$values.critical_error_channels == undefined ||
			!Array.isArray($values.critical_error_channels)
		) {
			$values.critical_error_channels = []
		}
		$values.critical_error_channels = $values.critical_error_channels.concat({ email: '' })
	}

	function removeChannel(index: number) {
		$values.critical_error_channels = $values.critical_error_channels.filter(
			(_: any, i: number) => i !== index
		)
	}

	function updateChannel(index: number, updatedChannel: any) {
		$values.critical_error_channels[index] = updatedChannel
	}

	function findChannelIndex(channel: any): number {
		return $values?.critical_error_channels?.indexOf(channel) ?? -1
	}

	function disconnectSlack() {
		// Clear the entire critical_error_channels setting, same as original disconnect behavior
		if ($values['slack']) {
			$values.slack = undefined
		}
	}

	function handleTeamChange(
		teamItem: { team_id: string; team_name: string } | undefined,
		channel: any
	) {
		const index = findChannelIndex(channel)
		if (index === -1) return

		const currentTeamChannel = channel?.teams_channel
		const teamIdChanged = currentTeamChannel?.team_id !== teamItem?.team_id

		$values.critical_error_channels[index] = {
			teams_channel: teamItem
				? {
						team_id: teamItem.team_id,
						team_name: teamItem.team_name,
						// Preserve existing channel if team didn't actually change
						channel_id: teamIdChanged ? undefined : currentTeamChannel?.channel_id,
						channel_name: teamIdChanged ? undefined : currentTeamChannel?.channel_name
					}
				: undefined
		}
	}

	function handleChannelChange(
		channelItem: { channel_id?: string; channel_name?: string } | undefined,
		channel: any
	) {
		const index = findChannelIndex(channel)
		if (index === -1) return

		const team = channel?.teams_channel
		if (team) {
			$values.critical_error_channels[index] = {
				teams_channel: {
					team_id: team?.team_id,
					team_name: team?.team_name,
					channel_id: channelItem?.channel_id,
					channel_name: channelItem?.channel_name
				}
			}
		}
	}
</script>

{#if !$enterpriseLicense}
	<EEOnly>Channels other than tracing are only available in the EE version</EEOnly>
{/if}

<div class="gap-y-4 pt-2 flex flex-col">
	<!-- Slack Card -->
	<SlackChannelCard
		channels={slackChannels}
		disabled={!$enterpriseLicense}
		onAddChannel={addSlackChannel}
		onRemoveChannel={removeChannel}
		onUpdateChannel={updateChannel}
		{findChannelIndex}
		onDisconnect={disconnectSlack}
		{slackTeamName}
		style="order: {slackOrder};"
	/>

	<!-- Teams Card -->
	<TeamsChannelCard
		channels={teamsChannels}
		disabled={!$enterpriseLicense}
		onAddChannel={addTeamsChannel}
		onRemoveChannel={removeChannel}
		onTeamChange={handleTeamChange}
		onChannelChange={handleChannelChange}
		{findChannelIndex}
		{isTeamsConnected}
		style="order: {teamsOrder};"
	/>

	<!-- Email Card -->
	<EmailChannelCard
		channels={emailChannels}
		hasSmtpConfig={isSmtpSettingsValid($values['smtp_settings'])}
		disabled={!$enterpriseLicense}
		onAddChannel={addEmailChannel}
		onRemoveChannel={removeChannel}
		onUpdateChannel={updateChannel}
		{openSmtpSettings}
		{findChannelIndex}
		style="order: {emailOrder};"
	/>
</div>
