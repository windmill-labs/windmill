<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'
	import CenteredPage from '$lib/components/CenteredPage.svelte'

	let error = $page.url.searchParams.get('error')
	let client_name = $page.url.searchParams.get('client_name')

	if (client_name) {
		sendUserToast(
			`Connection added for ${client_name}. The oauth token has been stored in the variables`
		)
	} else if (error) {
		sendUserToast(`Error adding connection ${error}`, true)
	} else {
		goto('/variables')
	}
	setTimeout(() => goto('/workspace_settings'), 5000)
</script>

<CenteredPage>
	<div class="py-6">
		<h1>Connection added for {client_name}</h1>
		<p>
			Redirecting in 5s to your workspace settings. <br />
			The oauth token has been stored in the variables at 'g/all/{client_name}' and also a resource
			of type {client_name} at same path 'g/all/{client_name}' refering to that token.
		</p>
	</div>
</CenteredPage>
