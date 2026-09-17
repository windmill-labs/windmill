<script lang="ts">
	import { page } from '$app/state'
	import { Button } from '$lib/components/common'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	// Mail scanners open links on delivery. Loading this page must never spend the link: only
	// the click below does.
	const token = page.url.searchParams.get('token') ?? ''

	let signingIn = $state(false)

	async function signIn() {
		if (signingIn) return
		signingIn = true
		try {
			const { location } = await UserService.confirmLoginLink({ token })
			// A full load, like the redirect a plain link gets, so the app starts from the new session.
			window.location.assign(location)
		} catch (e) {
			console.error('Could not sign in with the link:', e)
			sendUserToast('Could not sign in right now, please try again', true)
			signingIn = false
		}
	}
</script>

<CenteredModal
	title="Sign in to Windmill"
	subtitle="This link signs you in once, then stops working."
>
	{#if token}
		<Button variant="accent" unifiedSize="lg" loading={signingIn} onClick={signIn}>Sign in</Button>
	{:else}
		<p class="text-sm text-secondary">This sign-in link is not valid.</p>
	{/if}
</CenteredModal>
