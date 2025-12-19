<script lang="ts">
	import { Mail, X, Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import IntegrationCard from './IntegrationCard.svelte'
	import { fade } from 'svelte/transition'
	import TextInput from '../text_input/TextInput.svelte'
	import SmtpConfigurationStatus from '../common/smtp/SmtpConfigurationStatus.svelte'

	interface EmailChannel {
		email: string
	}

	interface Props {
		channels: EmailChannel[]
		disabled?: boolean
		onAddChannel: () => void
		onRemoveChannel: (index: number) => void
		onUpdateChannel: (index: number, updatedChannel: EmailChannel) => void
		findChannelIndex: (channel: EmailChannel) => number
		openSmtpSettings?: () => void
		class?: string
		style?: string
		hasSmtpConfig: boolean
	}

	let {
		channels,
		disabled = false,
		onAddChannel,
		onRemoveChannel,
		onUpdateChannel,
		findChannelIndex,
		openSmtpSettings,
		class: clazz,
		style,
		hasSmtpConfig
	}: Props = $props()

	function handleEmailInput(channel: EmailChannel, value: string) {
		const index = findChannelIndex(channel)
		if (index !== -1) {
			onUpdateChannel(index, { email: value })
		}
	}

	function handleRemoveChannel(channel: EmailChannel) {
		const index = findChannelIndex(channel)
		if (index !== -1) {
			onRemoveChannel(index)
		}
	}
</script>

{#if channels.length > 0}
	<!-- Connected Email Card -->
	<IntegrationCard title="Email" icon={Mail} isPlaceholder={false} class={clazz} {style}>
		{#snippet actions()}
			<SmtpConfigurationStatus {hasSmtpConfig} {openSmtpSettings} />
		{/snippet}
		{#snippet children()}
			{#if channels.length > 0}
				<span class="text-xs text-secondary"> Email addresses to send alerts to. </span>
			{/if}

			<!-- Email Inputs -->
			<div class="space-y-2">
				{#each channels as channel}
					<div class="flex items-center gap-2 w-full" transition:fade|local={{ duration: 200 }}>
						<TextInput
							inputProps={{
								type: 'email',
								placeholder: 'Email address',
								disabled,
								oninput: (e) => {
									const target = e.target as HTMLInputElement
									handleEmailInput(channel, target.value)
								}
							}}
							value={channel.email || ''}
						/>

						<button
							onclick={() => handleRemoveChannel(channel)}
							class="text-secondary hover:text-primary transition-colors"
							aria-label="Remove email"
							{disabled}
						>
							<X size={14} />
						</button>
					</div>
				{/each}
			</div>

			<!-- Add Email Button -->
			<div class="flex justify-start">
				<Button
					variant="default"
					size="xs"
					onclick={onAddChannel}
					btnClasses="text-xs flex items-center gap-2"
					{disabled}
				>
					<Plus size={14} />
					Add email address
				</Button>
			</div>
		{/snippet}
	</IntegrationCard>
{:else}
	<!-- Placeholder Card -->
	<IntegrationCard
		title="Email"
		icon={Mail}
		isPlaceholder={true}
		onAdd={onAddChannel}
		class={clazz}
		{style}
	>
		{#snippet children()}{/snippet}
	</IntegrationCard>
{/if}
