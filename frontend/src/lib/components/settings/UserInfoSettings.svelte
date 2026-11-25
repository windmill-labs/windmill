<script lang="ts">
	import { accountSetup } from '$lib/components/sidebar/accountSetup.svelte'
	import { usersWorkspaceStore } from '$lib/stores'
	import { UserService } from '$lib/gen'
	import { Alert, Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'
	import CopyButton from '../common/button/CopyButton.svelte'
	import { renderSVG } from 'uqr'

	let newPassword = $state<string | undefined>(undefined)
	let passwordError = $state<string | undefined>(undefined)
	let login_type = $state<string>('none')
	// A credential-less account may only take the password road where password login is
	// on; the backend refuses otherwise, this keeps the form from offering a dead end.
	let passwordAllowed = $state(true)

	$effect(() => {
		loadLoginType()
	})

	async function loadLoginType(): Promise<void> {
		login_type = (await UserService.globalWhoami()).login_type
		if (login_type == 'password') await loadTotp()
		if (login_type == 'pending_oauth') {
			passwordAllowed = !(await UserService.isPasswordLoginDisabled().catch(() => false))
		}
	}

	let totpEnabled = $state<boolean | undefined>(undefined)
	let totpSetup = $state<{ secret: string; otpauth_url: string } | undefined>(undefined)
	let totpCode = $state('')
	let recoveryCodes = $state<string[] | undefined>(undefined)

	async function loadTotp(): Promise<void> {
		// Builds without two-factor support do not serve the route: the section stays hidden.
		totpEnabled = (await UserService.getTotpStatus().catch(() => undefined))?.enabled
	}

	async function startTotpSetup(): Promise<void> {
		totpSetup = await UserService.setupTotp()
		totpCode = ''
	}

	async function submitTotp(action: 'enable' | 'disable' | 'recovery_codes'): Promise<void> {
		const requestBody = { code: totpCode }
		recoveryCodes = undefined
		if (action == 'disable') {
			sendUserToast(await UserService.disableTotp({ requestBody }))
		} else {
			recoveryCodes = (
				action == 'enable'
					? await UserService.enableTotp({ requestBody })
					: await UserService.regenerateTotpRecoveryCodes({ requestBody })
			).recovery_codes
			sendUserToast(
				action == 'enable' ? 'Two-factor authentication enabled' : 'New recovery codes created'
			)
		}
		totpSetup = undefined
		totpCode = ''
		await loadTotp()
	}

	async function setPassword(): Promise<void> {
		if (newPassword) {
			await UserService.setPassword({
				requestBody: {
					password: newPassword
				}
			})
			sendUserToast('Your password was successfully updated')
			await loadLoginType()
			// A credential-less account finishing here, rather than through the sidebar's
			// prompt, must clear that prompt too.
			await accountSetup.refresh()
		} else {
			sendUserToast('Specify a new password value to change your password', true)
		}
	}
</script>

<div class="border border-border-light rounded-md p-4 h-full">
	<h2 class="text-emphasis text-sm font-semibold mb-2">User info</h2>

	<form class="flex flex-col gap-6">
		<div class="w-full text-primary flex flex-col gap-1">
			<div class="text-xs text-emphasis font-semibold">Email</div>
			<span class="text-xs font-normal text-primary">
				{$usersWorkspaceStore?.email}
			</span>
		</div>

		<label class="flex flex-col gap-1 w-120">
			<span class="text-xs text-emphasis font-semibold">Password</span>
			{#if login_type == 'password' || (login_type == 'pending_oauth' && passwordAllowed)}
				<div class="flex flex-row gap-1 items-center">
					<TextInput
						inputProps={{ autocomplete: 'new-password', type: 'password' }}
						bind:value={newPassword}
						error={passwordError}
					/>
					<Button
						size="sm"
						variant="default"
						btnClasses="w-min whitespace-nowrap"
						on:click={setPassword}>Set password</Button
					>
				</div>
				{#if passwordError}
					<div class="text-red-600 text-2xs">{passwordError}</div>
				{/if}
			{:else if login_type == 'github'}
				<span class="text-xs text-primary font-normal"
					>Authenticated through Github OAuth2. Cannot set a password.</span
				>
			{/if}
		</label>

		{#if login_type == 'password' && totpEnabled != undefined}
			<div class="flex flex-col gap-1 w-120">
				<span class="text-xs text-emphasis font-semibold">Two-factor authentication</span>
				{#if totpEnabled}
					<span class="text-xs text-secondary font-normal">
						Logging in with your password also asks for a code from your authenticator app. Enter a
						current code or a recovery code to turn it off or to get new recovery codes.
					</span>
					{#if recoveryCodes}
						<div class="flex flex-col gap-1 p-2 rounded-md border bg-surface-secondary">
							<div class="flex flex-row gap-1 items-center text-xs text-emphasis font-semibold">
								Recovery codes
								<CopyButton value={recoveryCodes.join('\n')} />
							</div>
							<div class="grid grid-cols-2 gap-x-4 text-2xs font-mono font-normal text-emphasis">
								{#each recoveryCodes as code (code)}
									<span>{code}</span>
								{/each}
							</div>
							<Alert type="warning" title="Save these codes now" size="xs">
								Each one logs you in once if you lose your authenticator. They are shown only once.
							</Alert>
						</div>
					{/if}
				{:else if totpSetup}
					<span class="text-xs text-secondary font-normal">
						Scan this QR code with an authenticator app, or open it on this device, then enter the
						6-digit code it shows.
					</span>
					<a
						href={totpSetup.otpauth_url}
						aria-label="Add to an authenticator app"
						class="w-40 h-40 bg-white p-1 rounded-md"
					>
						{@html renderSVG(totpSetup.otpauth_url)}
					</a>
					<div class="flex flex-row gap-1 items-center text-2xs text-secondary font-normal">
						Or enter this key manually:
						<span class="font-mono text-emphasis">{totpSetup.secret}</span>
						<CopyButton value={totpSetup.secret} />
					</div>
				{:else}
					<span class="text-xs text-secondary font-normal">
						Ask for a code from an authenticator app each time you log in with your password.
					</span>
					<div>
						<Button unifiedSize="sm" variant="default" onClick={startTotpSetup}>
							Set up two-factor authentication
						</Button>
					</div>
				{/if}
				{#if totpEnabled || totpSetup}
					<div class="flex flex-row gap-1 items-center">
						<TextInput
							inputProps={{
								autocomplete: 'one-time-code',
								placeholder: '123456'
							}}
							bind:value={totpCode}
						/>
						{#if totpEnabled}
							<Button
								unifiedSize="sm"
								variant="default"
								btnClasses="w-min whitespace-nowrap"
								disabled={!totpCode}
								onClick={() => submitTotp('recovery_codes')}
							>
								New recovery codes
							</Button>
						{/if}
						<Button
							unifiedSize="sm"
							variant="default"
							btnClasses="w-min whitespace-nowrap"
							disabled={!totpCode}
							onClick={() => submitTotp(totpEnabled ? 'disable' : 'enable')}
						>
							{totpEnabled ? 'Turn off' : 'Turn on'}
						</Button>
					</div>
				{/if}
			</div>
		{/if}
	</form>
</div>
