<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { WindmillIcon } from '$lib/components/icons'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { UserService } from '$lib/gen'
	import LoginPageHeader from '$lib/components/LoginPageHeader.svelte'
	import { enterpriseLicense, whitelabelNameStore } from '$lib/stores'

	const token = $page.url.searchParams.get('token') ?? ''

	let newPassword = $state('')
	let confirmPassword = $state('')
	let loading = $state(false)
	let success = $state(false)

	async function resetPassword() {
		if (!token) {
			sendUserToast('Invalid or missing reset token', true)
			return
		}

		if (!newPassword || !confirmPassword) {
			sendUserToast('Please fill in both password fields', true)
			return
		}

		if (newPassword !== confirmPassword) {
			sendUserToast('Passwords do not match', true)
			return
		}

		loading = true
		try {
			await UserService.resetPassword({
				requestBody: {
					token,
					new_password: newPassword
				}
			})
			success = true
			sendUserToast('Password has been reset successfully!')
		} catch (err: any) {
			console.error('Could not reset password', err)
			sendUserToast('Could not reset password: ' + err, true)
		} finally {
			loading = false
		}
	}

	function handleKeyUp(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault()
			resetPassword()
		}
	}
</script>

<div
	class="flex flex-col justify-center py-12 sm:px-6 lg:px-8 relative bg-surface-secondary h-screen"
>
	<LoginPageHeader />
	<div class="sm:mx-auto sm:w-full sm:max-w-md">
		<div class="mx-auto flex justify-center">
			{#if !$enterpriseLicense || !$whitelabelNameStore}
				<WindmillIcon height="80px" width="80px" spin="slow" />
			{/if}
		</div>
		<h2 class="mt-6 text-center text-2xl font-semibold tracking-tight text-emphasis">
			{success ? 'Password Reset' : 'Set New Password'}
		</h2>
		{#if !success}
			<p class="mt-2 text-center text-xs text-secondary"> Enter your new password below </p>
		{/if}
	</div>

	<div class="mt-8 sm:mx-auto sm:w-full sm:max-w-xl mb-48">
		<div class="flex justify-end">
			<DarkModeToggle forcedDarkMode={false} />
		</div>
		<div class="bg-surface px-4 py-8 border sm:rounded-lg sm:px-10">
			{#if !token}
				<div class="text-center space-y-4">
					<p class="text-red-500">Invalid or missing reset token.</p>
					<div class="pt-4">
						<Button variant="accent" on:click={() => goto('/user/forgot-password')}>
							Request New Reset Link
						</Button>
					</div>
				</div>
			{:else if success}
				<div class="text-center space-y-4">
					<p class="text-secondary"> Your password has been reset successfully. </p>
					<p class="text-secondary text-sm"> You can now log in with your new password. </p>
					<div class="pt-4">
						<Button variant="accent" on:click={() => goto('/user/login')}>Go to login</Button>
					</div>
				</div>
			{:else}
				<div class="space-y-6">
					<div class="space-y-1">
						<label for="new-password" class="block text-xs font-semibold text-emphasis">
							New Password
						</label>
						<div>
							<input
								type="password"
								bind:value={newPassword}
								id="new-password"
								autocomplete="new-password"
								onkeyup={handleKeyUp}
							/>
						</div>
					</div>

					<div class="space-y-1">
						<label for="confirm-password" class="block text-xs font-semibold text-emphasis">
							Confirm Password
						</label>
						<div>
							<input
								type="password"
								bind:value={confirmPassword}
								id="confirm-password"
								autocomplete="new-password"
								onkeyup={handleKeyUp}
							/>
						</div>
					</div>

					<div class="pt-2 flex flex-col gap-2">
						<Button
							on:click={resetPassword}
							variant="accent"
							disabled={!newPassword || !confirmPassword || loading}
						>
							{loading ? 'Resetting...' : 'Reset password'}
						</Button>
						<Button variant="subtle" on:click={() => goto('/user/login')}>Back to login</Button>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
