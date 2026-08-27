<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import Button from '$lib/components/common/button/Button.svelte'
	import LoginPageHeader from '$lib/components/LoginPageHeader.svelte'

	const reason = page.url.searchParams.get('reason')

	const message = $derived(
		reason === 'used'
			? 'This sign-in link has already been used.'
			: reason === 'expired'
				? 'This sign-in link has expired.'
				: 'This sign-in link is not valid.'
	)
</script>

<div class="flex flex-col pt-24 pb-12 sm:px-6 lg:px-8 relative bg-surface-secondary min-h-screen">
	<LoginPageHeader />
	<div class="sm:mx-auto sm:w-full sm:max-w-sm">
		<h2 class="mt-6 text-center text-2xl font-semibold tracking-tight text-emphasis">
			Sign-in link unavailable
		</h2>
	</div>

	<div class="mt-6 sm:mx-auto sm:w-full sm:max-w-sm">
		<div class="bg-surface px-4 py-8 border sm:rounded-lg sm:px-10">
			<div class="text-center space-y-4">
				<p class="text-secondary">{message}</p>
				<p class="text-secondary text-sm">
					Sign-in links work once and for a few minutes. Ask for a new one from where you got this
					link, or sign in another way.
				</p>
				<div class="pt-4">
					<Button variant="accent" onClick={() => goto('/user/login')}>Go to login</Button>
				</div>
			</div>
		</div>
	</div>
</div>
