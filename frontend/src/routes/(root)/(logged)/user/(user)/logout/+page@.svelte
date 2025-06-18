<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import { clearUser } from '$lib/logout'
	import { onMount } from 'svelte'

	const rd = $page.url.searchParams.get('rd')

	onMount(async () => {
		try {
			await clearUser()
		} catch (err) {
			console.error(err)
		}
		if ($page.url.pathname != '/user/logout' && $page.url.pathname != '/user/login') {
			return
		}

		console.log('logout 4', rd)
		goto(rd ?? '/user/login')
	})
</script>

<CenteredModal title="Logging out">
	<div class="w-full">
		<div class="block m-auto w-20">
			<WindmillIcon height="80px" width="80px" spin="fast" />
		</div>
	</div>
</CenteredModal>
