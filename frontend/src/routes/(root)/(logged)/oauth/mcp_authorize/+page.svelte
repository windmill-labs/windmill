<script lang="ts">
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'

	// Get OAuth params from URL
	let workspaceId = $page.url.searchParams.get('workspace_id') || ''
	let clientId = $page.url.searchParams.get('client_id') || ''
	let clientName = $page.url.searchParams.get('client_name') || 'Unknown Client'
	let redirectUri = $page.url.searchParams.get('redirect_uri') || ''
	let scope = $page.url.searchParams.get('scope') || 'mcp:all'
	let oauthState = $page.url.searchParams.get('state') || ''
	let codeChallenge = $page.url.searchParams.get('code_challenge') || ''
	let codeChallengeMethod = $page.url.searchParams.get('code_challenge_method') || ''

	let loading = $state(false)
	let success = $state(false)
	let successRedirectUrl = $state('')

	function onDeny() {
		// Redirect to client with error
		const params = new URLSearchParams({
			error: 'access_denied',
			error_description: 'User denied the authorization request'
		})
		if (oauthState) {
			params.set('state', oauthState)
		}
		window.location.href = `${redirectUri}?${params.toString()}`
	}

	async function onApprove() {
		if (!workspaceId) {
			sendUserToast('Error: missing workspace_id', true)
			return
		}
		loading = true
		try {
			const approveUrl = `/api/w/${workspaceId}/mcp/oauth/server/approve`
			const response = await fetch(approveUrl, {
				method: 'POST',
				body: JSON.stringify({
					client_id: clientId,
					redirect_uri: redirectUri,
					scope: scope,
					state: oauthState,
					code_challenge: codeChallenge,
					code_challenge_method: codeChallengeMethod,
					approved: 'true'
				}),
				headers: {
					'Content-Type': 'application/json'
				}
			})
			if (response.ok) {
				const data = await response.json()
				// Include state in redirect if present
				const params = new URLSearchParams({ code: data.code })
				if (data.state) {
					params.set('state', data.state)
				}
				const url = `${redirectUri}?${params.toString()}`
				success = true
				successRedirectUrl = url
				loading = false
				window.location.href = url
			} else {
				sendUserToast('Error approving authorization request', true)
				loading = false
			}
		} catch (e) {
			sendUserToast('Error approving authorization request', true)
			loading = false
		}
	}
</script>

<CenteredModal title={success ? 'Authorization Approved' : 'Authorization Request'}>
	{#if success}
		<div class="text-center">
			<div class="mb-4 text-green-600 dark:text-green-400">
				<svg class="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M5 13l4 4L19 7"
					/>
				</svg>
			</div>
			<p class="text-lg mb-4">
				Authorization granted to <span class="font-semibold text-blue-600 dark:text-blue-400"
					>{clientName}</span
				>.
			</p>
			<p class="text-secondary text-sm mb-4">
				You should be redirected automatically. If not, click the link below:
			</p>
			<a
				href={successRedirectUrl}
				class="text-blue-600 dark:text-blue-400 hover:underline break-all"
			>
				{successRedirectUrl}
			</a>
		</div>
	{:else}
		<p class="text-center text-lg mb-6">
			<span class="font-semibold text-blue-600 dark:text-blue-400">{clientName}</span>
			is requesting access to your Windmill MCP tools
			{#if workspaceId}
				in workspace <span class="font-semibold text-blue-600 dark:text-blue-400"
					>{workspaceId}</span
				>
			{/if}
		</p>

		<div class="flex flex-row justify-around gap-x-4">
			<Button variant="border" size="lg" disabled={loading} onClick={onDeny}>Deny</Button>
			<Button variant="accent" size="lg" disabled={loading} {loading} onClick={onApprove}
				>Approve</Button
			>
		</div>
	{/if}
</CenteredModal>
