<script lang="ts" module>
	export function isSmtpSettingsValid(smtpSettings: Record<string, any>) {
		return (
			smtpSettings &&
			smtpSettings.smtp_host &&
			smtpSettings.smtp_host.trim() !== '' &&
			smtpSettings.smtp_port &&
			smtpSettings.smtp_username &&
			smtpSettings.smtp_username.trim() !== '' &&
			smtpSettings.smtp_password &&
			smtpSettings.smtp_password.trim() !== '' &&
			smtpSettings.smtp_from &&
			smtpSettings.smtp_from.trim() !== ''
		)
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/common'
	import Password from '../Password.svelte'
	import Toggle from '../Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'
	import { Mail } from 'lucide-svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	let testEmail = $state('')

	async function testSmtpSettings() {
		try {
			await SettingService.testSmtp({
				requestBody: {
					to: testEmail,
					smtp: {
						host: $values['smtp_settings'].smtp_host,
						username: $values['smtp_settings'].smtp_username,
						password: $values['smtp_settings'].smtp_password,
						port: $values['smtp_settings'].smtp_port,
						from: $values['smtp_settings'].smtp_from,
						tls_implicit: $values['smtp_settings'].smtp_tls_implicit || false,
						disable_tls: $values['smtp_settings'].smtp_disable_tls || false
					}
				}
			})
			sendUserToast('Test email sent successfully')
		} catch (error) {
			sendUserToast('Failed to send test email: ' + error.message, true)
		}
	}
</script>

<div class="space-y-6">
	<!-- SMTP Settings Form -->
	<div class="space-y-6">
		<div class="grid grid-cols-2 grid-rows-2 gap-x-2 gap-y-6">
			<div class="flex flex-col gap-1">
				<label for="smtp_host" class="block text-xs font-semibold text-emphasis mb-1">Host</label>
				<TextInput
					inputProps={{
						type: 'text',
						id: 'smtp_host',
						placeholder: 'smtp.gmail.com',
						disabled: disabled
					}}
					bind:value={$values['smtp_settings'].smtp_host}
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label for="smtp_port" class="block text-xs font-semibold text-emphasis mb-1">Port</label>
				<TextInput
					inputProps={{
						type: 'number',
						id: 'smtp_port',
						placeholder: '587',
						disabled: disabled
					}}
					bind:value={$values['smtp_settings'].smtp_port}
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
					bind:value={$values['smtp_settings'].smtp_username}
				/>
			</div>

			<div>
				<label for="smtp_password" class="block text-xs font-semibold text-emphasis mb-1">
					Password
				</label>
				<Password bind:password={$values['smtp_settings'].smtp_password} small {disabled} />
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
				bind:value={$values['smtp_settings'].smtp_from}
			/>
		</div>

		<div class="flex gap-4">
			<Toggle
				disabled={$values['smtp_settings'].smtp_disable_tls || disabled}
				id="smtp_tls_implicit"
				bind:checked={$values['smtp_settings'].smtp_tls_implicit}
				size="xs"
				options={{ right: 'Implicit TLS' }}
			/>

			<Toggle
				id="smtp_disable_tls"
				{disabled}
				bind:checked={$values['smtp_settings'].smtp_disable_tls}
				size="xs"
				on:change={(e) => {
					if (e.detail) {
						$values['smtp_settings'].smtp_tls_implicit = false
					}
				}}
				options={{ right: 'Disable TLS' }}
			/>
		</div>

		<!-- Test Email -->
		<div class="flex flex-col gap-1">
			<label for="test_email" class="block text-xs font-semibold text-emphasis">Test Email</label>
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
					unifiedSize="md"
					variant="accent"
					onclick={testSmtpSettings}
					disabled={!testEmail || !isSmtpSettingsValid($values['smtp_settings']) || disabled}
					btnClasses="text-xs"
					startIcon={{ icon: Mail }}
				>
					Send test email
				</Button>
			</div>
		</div>
	</div>
</div>
