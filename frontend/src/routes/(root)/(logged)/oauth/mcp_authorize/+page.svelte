<script lang="ts">
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'

	// Get OAuth params from URL
	let clientId = $page.url.searchParams.get('client_id') || ''
	let clientName = $page.url.searchParams.get('client_name') || 'Unknown Client'
	let redirectUri = $page.url.searchParams.get('redirect_uri') || ''
	let scope = $page.url.searchParams.get('scope') || 'mcp:all'
	let state = $page.url.searchParams.get('state') || ''
	let codeChallenge = $page.url.searchParams.get('code_challenge') || ''
	let codeChallengeMethod = $page.url.searchParams.get('code_challenge_method') || ''

	function onDeny() {
		// Redirect to client with error
		const params = new URLSearchParams({
			error: 'access_denied',
			error_description: 'User denied the authorization request'
		})
		if (state) {
			params.set('state', state)
		}
		window.location.href = `${redirectUri}?${params.toString()}`
	}

	async function onApprove() {
		// do POST request to /api/mcp/oauth/server/approve
		const response = await fetch('/api/mcp/oauth/server/approve', {
			method: 'POST',
			body: JSON.stringify({
				client_id: clientId,
				redirect_uri: redirectUri,
				scope: scope,
				state: state,
				code_challenge: codeChallenge,
				code_challenge_method: codeChallengeMethod,
				approved: 'true'
			}),
			headers: {
				'Content-Type': 'application/json'
			}
		})
		console.log(response)
		if (response.ok) {
			const data = await response.json()
			// Include state in redirect if present
			const params = new URLSearchParams({ code: data.code })
			if (data.state) {
				params.set('state', data.state)
			}
			window.location.href = `${redirectUri}?${params.toString()}`
		} else {
			sendUserToast('Error approving authorization request', true)
		}
	}
</script>

<CenteredModal title="Authorization Request">
	<p class="text-center text-lg mb-6">
		<span class="font-semibold text-blue-600 dark:text-blue-400">{clientName}</span>
		is requesting access to your Windmill MCP tools.
	</p>

	<div class="flex flex-row justify-around gap-x-4">
		<Button variant="border" size="lg" onClick={onDeny}>Deny</Button>
		<Button variant="accent" size="lg" onClick={onApprove}>Approve</Button>
	</div>
</CenteredModal>
