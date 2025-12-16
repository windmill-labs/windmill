<script lang="ts">
	import { X, Plus } from 'lucide-svelte'
	import MSTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'
	import IntegrationCard from './IntegrationCard.svelte'
	import TeamSelector from '../TeamSelector.svelte'
	import ChannelSelector from '../ChannelSelector.svelte'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'

	interface TeamsChannelEntry {
		teams_channel?: {
			team_id: string
			team_name: string
			channel_id?: string
			channel_name?: string
		}
	}

	interface Props {
		channels: TeamsChannelEntry[]
		disabled?: boolean
		onAddChannel: () => void
		onRemoveChannel: (index: number) => void
		onTeamChange: (
			teamItem: { team_id: string; team_name: string } | undefined,
			channel: TeamsChannelEntry
		) => void
		onChannelChange: (
			channelItem: { channel_id?: string; channel_name?: string } | undefined,
			channel: TeamsChannelEntry
		) => void
		findChannelIndex: (channel: TeamsChannelEntry) => number
		class?: string
		style?: string
	}

	let {
		channels,
		disabled = false,
		onAddChannel,
		onRemoveChannel,
		onTeamChange,
		onChannelChange,
		findChannelIndex,
		class: clazz,
		style
	}: Props = $props()

	function handleRemoveChannel(channel: TeamsChannelEntry) {
		const index = findChannelIndex(channel)
		if (index !== -1) {
			onRemoveChannel(index)
		}
	}
</script>

{#if channels.length > 0}
	<!-- Connected Teams Card -->
	<IntegrationCard
		title="Microsoft Teams"
		icon={MSTeamsIcon}
		hasChannels={true}
		isPlaceholder={false}
		class={clazz}
		{style}
	>
		{#snippet children()}
			<!-- Channel Configuration -->
			{#if channels.length > 0}
				<!-- Column Headers -->
				<div class="flex items-center gap-2 w-full">
					<div class="flex flex-row gap-2 flex-1">
						<div class="w-44">
							<span class="block text-xs font-normal text-secondary">Team</span>
						</div>
						<div class="flex-1">
							<span class="block text-xs font-normal text-secondary">Channel</span>
						</div>
					</div>
					<div class="w-6"></div>
					<!-- Space for remove button -->
				</div>
			{/if}
			<div class="space-y-2">
				{#each channels as channel}
					{@const currentTeam = channel?.teams_channel
						? {
								team_id: channel.teams_channel.team_id,
								team_name: channel.teams_channel.team_name
							}
						: undefined}
					{@const currentChannel = channel?.teams_channel?.channel_id
						? {
								channel_id: channel.teams_channel.channel_id,
								channel_name: channel.teams_channel.channel_name
							}
						: undefined}

					<div class="flex items-center gap-2 w-full">
						<div class="flex flex-row gap-2 flex-1">
							<TeamSelector
								containerClass="w-44"
								minWidth="140px"
								showRefreshButton={false}
								selectedTeam={currentTeam}
								onSelectedTeamChange={(team) => onTeamChange(team, channel)}
								{disabled}
							/>

							{#if channel?.teams_channel?.team_id}
								<ChannelSelector
									containerClass="flex-1"
									placeholder="Search channels"
									teamId={channel.teams_channel.team_id}
									selectedChannel={currentChannel}
									onSelectedChannelChange={(channelItem) => onChannelChange(channelItem, channel)}
									{disabled}
									onError={(e) => sendUserToast('Failed to load channels: ' + e.message, true)}
								/>
							{/if}
						</div>

						<button
							onclick={() => handleRemoveChannel(channel)}
							class="text-secondary hover:text-primary transition-colors"
							aria-label="Remove channel"
							{disabled}
						>
							<X size={14} />
						</button>
					</div>
				{/each}
			</div>

			<!-- Add Channel Button -->
			<div class="flex justify-start">
				<Button
					variant="default"
					size="xs"
					onclick={onAddChannel}
					btnClasses="text-xs flex items-center gap-2"
					{disabled}
				>
					<Plus size={14} />
					Add Teams channel
				</Button>
			</div>
		{/snippet}
	</IntegrationCard>
{:else}
	<!-- Placeholder Card -->
	<IntegrationCard
		title="Microsoft Teams"
		icon={MSTeamsIcon}
		hasChannels={false}
		isPlaceholder={true}
		onAdd={onAddChannel}
		class={clazz}
		{style}
	>
		{#snippet children()}{/snippet}
	</IntegrationCard>
{/if}
