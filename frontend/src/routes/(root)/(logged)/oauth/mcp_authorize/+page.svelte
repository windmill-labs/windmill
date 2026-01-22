<script lang="ts">
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { Check, Info } from 'lucide-svelte'

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
					code_challenge_method: codeChallengeMethod
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
				const errorData = await response.json().catch(() => null)
				const errorMsg = errorData?.message || `Server returned ${response.status}`
				sendUserToast(`Error: ${errorMsg}`, true)
				loading = false
			}
		} catch (e) {
			sendUserToast('Error approving authorization request', true)
			loading = false
		}
	}
</script>

{#if !workspaceId}
	<p class="text-center text-sm text-primary mb-6"> Error: missing workspace_id </p>
{:else}
	<CenteredModal title={success ? 'Authorization Approved' : 'Authorization Request'}>
		{#if success}
			<div class="text-center">
				<div class="mb-4 text-green-500">
					<Check class="w-16 h-16 mx-auto" />
				</div>
				<p class="text-sm text-primary mb-4">
					Authorization granted to <span class="font-semibold text-accent">{clientName}</span>.
				</p>
				<p class="text-xs text-secondary mb-4">
					You should be redirected automatically. If not, click the link below:
				</p>
				<a href={successRedirectUrl} class="text-xs text-accent hover:underline break-all">
					{successRedirectUrl}
				</a>
			</div>
		{:else}
			<p class="text-center text-sm text-primary mb-6">
				<span class="font-semibold text-accent">{clientName}</span>
				is requesting access to your
				<span class="font-semibold text-accent">{workspaceId}</span>
				workspace.
			</p>

			<div class="mb-6">
				<p class="text-xs font-semibold text-emphasis mb-3">This will allow the client to:</p>
				<ul class="flex flex-col gap-y-2">
					<li class="flex items-center gap-x-2 text-xs text-primary">
						<Check class="w-4 h-4 text-green-500 flex-shrink-0" />
						Execute all scripts in the workspace
					</li>
					<li class="flex items-center gap-x-2 text-xs text-primary">
						<Check class="w-4 h-4 text-green-500 flex-shrink-0" />
						Execute all flows in the workspace
					</li>
					<li class="flex items-center gap-x-2 text-xs text-primary">
						<Check class="w-4 h-4 text-green-500 flex-shrink-0" />
						Access API endpoints related to the workspace
					</li>
				</ul>
			</div>

			<div
				class="flex items-start gap-x-2 p-3 mb-6 rounded-md bg-surface-secondary border border-light"
			>
				<Info class="w-4 h-4 text-secondary flex-shrink-0 mt-0.5" />
				<p class="text-2xs text-secondary">
					For more fine-grained control, you can create a specific token with limited scope from
					your account settings.
					<a
						href="https://www.windmill.dev/docs/core_concepts/mcp"
						target="_blank"
						rel="noopener noreferrer"
						class="text-accent hover:underline">See documentation</a
					>.
				</p>
			</div>

			<div class="flex flex-row justify-around gap-x-4">
				<Button variant="border" size="lg" disabled={loading} onClick={onDeny}>Deny</Button>
				<Button variant="accent" size="lg" disabled={loading} {loading} onClick={onApprove}
					>Approve</Button
				>
			</div>
		{/if}
	</CenteredModal>
{/if}
