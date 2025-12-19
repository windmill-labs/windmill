<script lang="ts">
	import { Slack, X, Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import IntegrationCard from './IntegrationCard.svelte'
	import SlackConnectionStatus from '../common/slack/SlackConnectionStatus.svelte'
	import TextInput from '../text_input/TextInput.svelte'

	interface SlackChannel {
		slack_channel: string
	}

	interface Props {
		channels: SlackChannel[]
		disabled?: boolean
		onAddChannel: () => void
		onRemoveChannel: (index: number) => void
		onUpdateChannel: (index: number, updatedChannel: SlackChannel) => void
		findChannelIndex: (channel: SlackChannel) => number
		onDisconnect: () => void
		slackTeamName?: string
		class?: string
		style?: string
	}

	let {
		channels,
		disabled = false,
		onAddChannel,
		onRemoveChannel,
		onUpdateChannel,
		findChannelIndex,
		onDisconnect,
		slackTeamName,
		class: clazz,
		style
	}: Props = $props()

	function handleSlackChannelInput(channel: SlackChannel, value: string) {
		const index = findChannelIndex(channel)
		if (index !== -1) {
			onUpdateChannel(index, { slack_channel: value })
		}
	}

	function handleRemoveChannel(channel: SlackChannel) {
		const index = findChannelIndex(channel)
		if (index !== -1) {
			onRemoveChannel(index)
		}
	}
</script>

{#if channels.length > 0}
	<!-- Connected Slack Card -->
	<IntegrationCard title="Slack" icon={Slack} isPlaceholder={false} class={clazz} {style}>
		{#snippet actions()}
			<SlackConnectionStatus
				isConnected={slackTeamName ? true : false}
				{slackTeamName}
				mode="instance"
				{onDisconnect}
			/>
		{/snippet}
		{#snippet children()}
			{#if channels.length > 0}
				<span class="text-xs text-secondary"> Channels to send alerts to. </span>
			{/if}

			<!-- Channel Inputs -->
			<div class="space-y-2">
				{#each channels as channel}
					<div class="flex items-center gap-2 w-full">
						<div class="flex-1">
							<TextInput
								inputProps={{
									type: 'text',
									placeholder: 'Slack channel (e.g., #general)',
									disabled: disabled,
									oninput: (e) => {
										const target = e.target as HTMLInputElement
										handleSlackChannelInput(channel, target.value)
									}
								}}
								value={channel.slack_channel || ''}
							/>
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
					Add channel
				</Button>
			</div>
		{/snippet}
	</IntegrationCard>
{:else}
	<!-- Placeholder Card -->
	<IntegrationCard
		title="Slack"
		icon={Slack}
		isPlaceholder={true}
		onAdd={onAddChannel}
		class={clazz}
		{style}
	>
		{#snippet children()}{/snippet}
	</IntegrationCard>
{/if}
