<script lang="ts">
	import { Mail, X, Plus, ExternalLink } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import IntegrationCard from './IntegrationCard.svelte'
	import Password from '../Password.svelte'
	import Toggle from '../Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { fade, slide } from 'svelte/transition'
	import TextInput from '../text_input/TextInput.svelte'

	interface EmailChannel {
		email: string
	}

	interface Props {
		channels: EmailChannel[]
		smtpSettings: Record<string, any>
		disabled?: boolean
		onAddChannel: () => void
		onRemoveChannel: (index: number) => void
		onUpdateChannel: (index: number, updatedChannel: EmailChannel) => void
		findChannelIndex: (channel: EmailChannel) => number
		class?: string
		style?: string
	}

	let {
		channels,
		smtpSettings,
		disabled = false,
		onAddChannel,
		onRemoveChannel,
		onUpdateChannel,
		findChannelIndex,
		class: clazz,
		style
	}: Props = $props()

	let expanded = $state(false)
	let testEmail = $state('')

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

	function toggleExpanded() {
		expanded = !expanded
	}

	async function testSmtpSettings() {
		if (!testEmail) return

		try {
			await SettingService.testSmtp({
				requestBody: {
					to: testEmail,
					smtp: {
						host: smtpSettings['smtp_host'],
						username: smtpSettings['smtp_username'],
						password: smtpSettings['smtp_password'],
						port: smtpSettings['smtp_port'],
						from: smtpSettings['smtp_from'],
						tls_implicit: smtpSettings['smtp_tls_implicit'],
						disable_tls: smtpSettings['smtp_disable_tls']
					}
				}
			})
			sendUserToast('Test email sent successfully')
		} catch (error) {
			sendUserToast('Failed to send test email: ' + error.message, true)
		}
	}

	let hasSmtpConfig = $derived(
		smtpSettings && smtpSettings.smtp_host && smtpSettings.smtp_host !== ''
	)
</script>

{#if channels.length > 0 || hasSmtpConfig}
	<!-- Connected Email Card -->
	<IntegrationCard
		title="Email"
		icon={Mail}
		hasChannels={true}
		isPlaceholder={false}
		expandable={true}
		{expanded}
		forceExpanded={!hasSmtpConfig}
		onToggleExpand={toggleExpanded}
		class={clazz}
		{style}
	>
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

			<!-- Expandable SMTP Configuration -->
			{#if expanded || !hasSmtpConfig}
				<div transition:slide={{ duration: 200 }}>
					<div class="border-t pt-2">
						<div class="mb-6">
							<h4 class="text-sm font-semibold text-primary mb-1">SMTP Configuration</h4>
							<p class="text-xs text-secondary"
								>Configure SMTP settings for sending email alerts. <a
									href="https://www.windmill.dev/docs/advanced/instance_settings#smtp"
									>Learn more <ExternalLink size={12} class="inline-block" /></a
								></p
							>
						</div>

						<!-- SMTP Settings Form -->
						<div class="space-y-6">
							<div class="grid grid-cols-2 grid-rows-2 gap-x-2 gap-y-6">
								<div class="flex flex-col gap-1">
									<label for="smtp_host" class="block text-xs font-semibold text-emphasis mb-1">
										Host
									</label>
									<TextInput
										inputProps={{
											type: 'text',
											id: 'smtp_host',
											placeholder: 'smtp.gmail.com',
											disabled: disabled
										}}
										bind:value={smtpSettings.smtp_host}
									/>
								</div>
								<div class="flex flex-col gap-1">
									<label for="smtp_port" class="block text-xs font-semibold text-emphasis mb-1">
										Port
									</label>
									<TextInput
										inputProps={{
											type: 'number',
											id: 'smtp_port',
											placeholder: '587',
											disabled: disabled
										}}
										bind:value={smtpSettings.smtp_port}
									/>
								</div>

								<div>
									<label for="smtp_username" class="block text-xs font-semibold text-emphasis mb-1">
										Username
									</label>
									<TextInput
										inputProps={{
											type: 'text',
											id: 'smtp_username',
											placeholder: 'user@example.com',
											disabled: disabled
										}}
										bind:value={smtpSettings.smtp_username}
									/>
								</div>

								<div>
									<label for="smtp_password" class="block text-xs font-semibold text-emphasis mb-1">
										Password
									</label>
									<Password bind:password={smtpSettings.smtp_password} small {disabled} />
								</div>
							</div>

							<div>
								<label for="smtp_from" class="block text-xs font-semibold text-emphasis mb-1">
									From Address
								</label>
								<TextInput
									inputProps={{
										type: 'email',
										id: 'smtp_from',
										placeholder: 'noreply@example.com',
										disabled: disabled
									}}
									bind:value={smtpSettings.smtp_from}
								/>
							</div>

							<div class="flex gap-4">
								<Toggle
									disabled={smtpSettings.smtp_disable_tls || disabled}
									id="smtp_tls_implicit"
									bind:checked={smtpSettings.smtp_tls_implicit}
									size="xs"
									options={{ right: 'Implicit TLS' }}
								/>

								<Toggle
									id="smtp_disable_tls"
									{disabled}
									bind:checked={smtpSettings.smtp_disable_tls}
									size="xs"
									on:change={() => {
										if (smtpSettings.smtp_disable_tls) {
											smtpSettings.smtp_tls_implicit = false
										}
									}}
									options={{ right: 'Disable TLS' }}
								/>
							</div>

							<!-- Test Email -->
							<div class="flex flex-col gap-1">
								<label for="test_email" class="block text-xs font-semibold text-emphasis"
									>Test Email</label
								>
								<span class="text-xs text-secondary">
									Enter a test email address to verify the SMTP settings.
								</span>
								<div class="flex gap-2">
									<TextInput
										inputProps={{
											type: 'email',
											placeholder: 'Test email address',
											disabled: disabled,
											id: 'test_email'
										}}
										bind:value={testEmail}
									/>
									<Button
										size="xs"
										onclick={testSmtpSettings}
										disabled={!testEmail || disabled}
										btnClasses="text-xs"
									>
										Test SMTP
									</Button>
								</div>
							</div>
						</div>
					</div>
				</div>
			{:else if expanded && !hasSmtpConfig}
				<div transition:slide|local={{ duration: 300 }}>
					<div class="border-t border-gray-600 pt-3 mt-3">
						<div class="text-center py-3">
							<p class="text-xs text-secondary">
								SMTP configuration not found. Please configure SMTP settings to send email alerts.
							</p>
						</div>
					</div>
				</div>
			{/if}
		{/snippet}
	</IntegrationCard>
{:else}
	<!-- Placeholder Card -->
	<IntegrationCard
		title="Email"
		icon={Mail}
		hasChannels={false}
		isPlaceholder={true}
		onAdd={onAddChannel}
		class={clazz}
		{style}
	>
		{#snippet children()}{/snippet}
	</IntegrationCard>
{/if}
