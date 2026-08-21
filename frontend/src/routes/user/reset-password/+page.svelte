<script lang="ts">
	import { goto } from '$lib/navigation'
	import { tick } from 'svelte'
	import { page } from '$app/state'
	import Button from '$lib/components/common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { UserService } from '$lib/gen'
	import LoginPageHeader from '$lib/components/LoginPageHeader.svelte'
	import Password from '$lib/components/Password.svelte'

	const token = page.url.searchParams.get('token') ?? ''

	let newPassword = $state('')
	let confirmPassword = $state('')
	let loading = $state(false)
	let success = $state(false)
	let newPasswordField = $state<Password | undefined>(undefined)
	let confirmPasswordField = $state<Password | undefined>(undefined)

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

		// Await the DOM update: the fields must be back to type="password" before the
		// request goes out, or the browser may not offer to save the new credential
		newPasswordField?.conceal()
		confirmPasswordField?.conceal()
		await tick()

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

	function handleKeyDown(event: KeyboardEvent) {
		// keydown auto-repeats while held, and Enter also confirms an IME candidate — either
		// would fire several resets against a single-use token
		if (event.key === 'Enter' && !event.isComposing && !event.repeat) {
			event.preventDefault()
			resetPassword()
		}
	}
</script>

<!-- Anchored to the top, not centered: the card grows when the password form opens or an
	error appears, and centering would slide the mark and the fields under the pointer. -->
<div class="flex flex-col pt-24 pb-12 sm:px-6 lg:px-8 relative bg-surface-secondary min-h-screen">
	<LoginPageHeader />
	<div class="sm:mx-auto sm:w-full sm:max-w-sm">
		<h2 class="mt-6 text-center text-2xl font-semibold tracking-tight text-emphasis">
			{success ? 'Password Reset' : 'Set New Password'}
		</h2>
		{#if !success}
			<p class="mt-2 text-center text-xs text-secondary"> Enter your new password below </p>
		{/if}
	</div>

	<div class="mt-6 sm:mx-auto sm:w-full sm:max-w-sm">
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
							<Password
								bind:this={newPasswordField}
								bind:password={newPassword}
								id="new-password"
								placeholder=""
								allowMultiline={false}
								onKeyDown={(e) => {
									if (e.key === 'Enter' && !e.isComposing && !e.repeat) {
										e.preventDefault()
										confirmPasswordField?.focus()
									}
								}}
							/>
						</div>
					</div>

					<div class="space-y-1">
						<label for="confirm-password" class="block text-xs font-semibold text-emphasis">
							Confirm Password
						</label>
						<div>
							<Password
								bind:this={confirmPasswordField}
								bind:password={confirmPassword}
								id="confirm-password"
								placeholder=""
								allowMultiline={false}
								onKeyDown={handleKeyDown}
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
