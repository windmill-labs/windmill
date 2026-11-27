<script lang="ts">
	import { accountSetup } from '$lib/components/sidebar/accountSetup.svelte'
	import { usersWorkspaceStore } from '$lib/stores'
	import { UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'
	import TotpModal, { type TotpAction } from './TotpModal.svelte'

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
	let totpRequired = $state(false)
	let totpAction = $state<TotpAction | undefined>(undefined)

	async function loadTotp(): Promise<void> {
		// Builds without two-factor support do not serve the route: the section stays hidden.
		const status = await UserService.getTotpStatus().catch(() => undefined)
		totpEnabled = status?.enabled
		totpRequired = status?.required ?? false
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

<form class="flex flex-col gap-6">
	<div class="w-full text-primary flex flex-col gap-1">
		<div class="text-xs text-emphasis font-semibold">Email</div>
		<span class="text-xs font-normal text-primary">
			{$usersWorkspaceStore?.email}
		</span>
	</div>

	{#if login_type == 'password' || (login_type == 'pending_oauth' && passwordAllowed)}
		<label class="flex flex-col gap-1 w-120">
			<span class="text-xs text-emphasis font-semibold">Password</span>
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
		</label>
	{:else if !['none', 'pending_oauth', 'service_account'].includes(login_type)}
		<div class="flex flex-col gap-1">
			<div class="text-xs text-emphasis font-semibold">Sign-in</div>
			<span class="text-xs font-normal text-primary">
				Through <span class="capitalize">{login_type}</span> single sign-on
			</span>
		</div>
	{/if}

	{#if login_type == 'password' && totpEnabled != undefined}
		<div class="flex flex-col gap-1 w-120">
			<span class="text-xs text-emphasis font-semibold">Two-factor authentication</span>
			<span class="text-xs text-secondary font-normal">
				{totpEnabled
					? totpRequired
						? 'On, as this instance requires. Logging in with your password also asks for a code from your authenticator app.'
						: 'On. Logging in with your password also asks for a code from your authenticator app.'
					: 'Ask for a code from an authenticator app each time you log in with your password.'}
			</span>
			<div class="flex flex-row gap-1">
				{#if totpEnabled}
					<Button
						unifiedSize="sm"
						variant="default"
						onClick={() => (totpAction = 'recovery_codes')}
					>
						New recovery codes
					</Button>
					{#if !totpRequired}
						<Button unifiedSize="sm" variant="default" onClick={() => (totpAction = 'disable')}>
							Turn off
						</Button>
					{/if}
				{:else}
					<Button unifiedSize="sm" variant="default" onClick={() => (totpAction = 'setup')}>
						Set up two-factor authentication
					</Button>
				{/if}
			</div>
		</div>
	{/if}
</form>
<TotpModal bind:action={totpAction} onClose={loadTotp} />
