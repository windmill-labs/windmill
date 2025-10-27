<script lang="ts">
	import { usersWorkspaceStore } from '$lib/stores'
	import { UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'

	let newPassword = $state<string | undefined>(undefined)
	let passwordError = $state<string | undefined>(undefined)
	let login_type = $state<string>('none')

	$effect(() => {
		loadLoginType()
	})

	async function loadLoginType(): Promise<void> {
		login_type = (await UserService.globalWhoami()).login_type
	}

	async function setPassword(): Promise<void> {
		if (newPassword) {
			await UserService.setPassword({
				requestBody: {
					password: newPassword
				}
			})
			sendUserToast('Your password was successfully updated')
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
			{#if login_type == 'password'}
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
	</form>
</div>
