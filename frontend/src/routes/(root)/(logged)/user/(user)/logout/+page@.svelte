<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
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

		if (rd?.startsWith('https://') || rd?.startsWith('http://')) {
			window.location.href = rd
		} else {
			goto(rd ?? '/user/login')
		}
	})
</script>

<CenteredModal title="Logging out" loading={true}></CenteredModal>
