<script lang="ts">
	import { accountSetup } from '$lib/components/sidebar/accountSetup.svelte'
	import { usersWorkspaceStore } from '$lib/stores'
	import { UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'

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
		if (login_type == 'pending_oauth') {
			passwordAllowed = !(await UserService.isPasswordLoginDisabled().catch(() => false))
		}
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
	{:else if login_type != 'none' && login_type != 'pending_oauth'}
		<div class="flex flex-col gap-1">
			<div class="text-xs text-emphasis font-semibold">Sign-in</div>
			<span class="text-xs font-normal text-primary">
				Through <span class="capitalize">{login_type}</span> single sign-on
			</span>
		</div>
	{/if}
</form>
