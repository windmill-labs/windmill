<script lang="ts">
	import { usersWorkspaceStore } from '$lib/stores'
	import { UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'

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

<div>
	<h2 class="border-b">User info</h2>
	<div class="">
		{#if passwordError}
			<div class="text-red-600 text-2xs grow">{passwordError}</div>
		{/if}
		<div class="flex flex-col gap-2 w-full">
			<div class="mt-4">
				<label class="block w-60 mb-2 text-tertiary">
					<div class="text-secondary">email</div>
					<input type="text" disabled value={$usersWorkspaceStore?.email} class="input mt-1" />
				</label>
				{#if login_type == 'password'}
					<label class="block w-120">
						<div class="text-secondary">password</div>
						<input
							type="password"
							bind:value={newPassword}
							class="
							w-full
							block
							py-1
							px-2
							rounded-md
							border
							border-gray-300
							shadow-sm
							focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
							text-sm
							"
						/>
						<Button size="sm" btnClasses="mt-4 w-min" on:click={setPassword}>Set password</Button>
					</label>
				{:else if login_type == 'github'}
					<span class="text-sm">Authenticated through Github OAuth2. Cannot set a password.</span>
				{/if}
			</div>
		</div>
	</div>
</div>
