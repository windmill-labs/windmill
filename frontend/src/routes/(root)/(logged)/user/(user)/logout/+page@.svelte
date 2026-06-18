<script lang="ts">
	import { page } from '$app/state'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { clearUser } from '$lib/logout'
	import { isValidLogoutRedirect } from '$lib/logoutRedirect'
	import { userStore } from '$lib/stores'
	import { onMount } from 'svelte'

	const rd = page.url.searchParams.get('rd')

	function sleep(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms))
	}

	onMount(async () => {
		try {
			await clearUser()
		} catch (err) {
			console.error(err)
		}

		userStore.set(undefined)

		const { OpenAPI } = await import('$lib/gen')
		OpenAPI.TOKEN = undefined

		await sleep(3000)

		if (page.url.pathname != '/user/logout' && page.url.pathname != '/user/login') {
			return
		}

		if (rd && isValidLogoutRedirect(rd)) {
			window.location.href = rd
		} else {
			window.location.href = '/user/login'
		}
	})
</script>

<CenteredModal title="Logging out" loading={true}></CenteredModal>
