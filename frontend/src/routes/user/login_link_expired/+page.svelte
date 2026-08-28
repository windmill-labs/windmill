<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { Button } from '$lib/components/common'
	import CenteredModal from '$lib/components/CenteredModal.svelte'

	const reason = $derived(page.url.searchParams.get('reason'))

	const message = $derived(
		reason === 'used'
			? 'This sign-in link has already been used.'
			: reason === 'expired'
				? 'This sign-in link has expired.'
				: 'This sign-in link is not valid.'
	)
</script>

<CenteredModal
	title="Sign-in link unavailable"
	subtitle="{message} Sign-in links work once and for a few minutes: ask for a new one from where you got this link, or sign in another way."
>
	<Button variant="accent" unifiedSize="lg" onClick={() => goto('/user/login')}>Go to login</Button>
</CenteredModal>
