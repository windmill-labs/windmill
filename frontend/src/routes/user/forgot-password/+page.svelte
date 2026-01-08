<script lang="ts">
	import { goto } from '$lib/navigation'
	import { WindmillIcon } from '$lib/components/icons'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { UserService } from '$lib/gen'
	import LoginPageHeader from '$lib/components/LoginPageHeader.svelte'
	import { enterpriseLicense, whitelabelNameStore } from '$lib/stores'

	let email = $state('')
	let loading = $state(false)
	let submitted = $state(false)

	async function requestPasswordReset() {
		if (!email) {
			sendUserToast('Please enter your email address', true)
			return
		}

		loading = true
		try {
			await UserService.requestPasswordReset({ requestBody: { email } })
			submitted = true
			sendUserToast('If an account with that email exists, a password reset link has been sent.')
		} catch (err: any) {
			if (err?.body?.includes('SMTP is not configured')) {
				sendUserToast('Password reset is not available. SMTP is not configured.', true)
			} else {
				sendUserToast('An error occurred. Please try again later.', true)
			}
		} finally {
			loading = false
		}
	}

	function handleKeyUp(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault()
			requestPasswordReset()
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
			Reset Password
		</h2>
		<p class="mt-2 text-center text-xs text-secondary">
			Enter your email address and we'll send you a link to reset your password
		</p>
	</div>

	<div class="mt-8 sm:mx-auto sm:w-full sm:max-w-xl mb-48">
		<div class="flex justify-end">
			<DarkModeToggle forcedDarkMode={false} />
		</div>
		<div class="bg-surface px-4 py-8 border sm:rounded-lg sm:px-10">
			{#if submitted}
				<div class="text-center space-y-4">
					<p class="text-secondary">
						If an account with that email exists, we've sent a password reset link.
					</p>
					<p class="text-secondary text-sm">
						Please check your email and follow the instructions to reset your password.
					</p>
					<div class="pt-4">
						<Button variant="accent" on:click={() => goto('/user/login')}>
							Back to Login
						</Button>
					</div>
				</div>
			{:else}
				<div class="space-y-6">
					<div class="space-y-1">
						<label for="email" class="block text-xs font-semibold text-emphasis">Email</label>
						<div>
							<input
								type="email"
								bind:value={email}
								id="email"
								autocomplete="email"
								onkeyup={handleKeyUp}
							/>
						</div>
					</div>

					<div class="pt-2 flex flex-col gap-2">
						<Button on:click={requestPasswordReset} variant="accent" disabled={!email || loading}>
							{loading ? 'Sending...' : 'Send Reset Link'}
						</Button>
						<Button variant="subtle" on:click={() => goto('/user/login')}>
							Back to Login
						</Button>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
