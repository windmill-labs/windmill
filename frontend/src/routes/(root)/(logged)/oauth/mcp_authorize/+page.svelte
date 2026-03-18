<script lang="ts">
	import { page } from '$app/state'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import McpScopeSelector from '$lib/components/mcp/McpScopeSelector.svelte'
	import Check from 'lucide-svelte/icons/check'

	// Get OAuth params from URL
	let isGateway = page.url.searchParams.get('gateway') === 'true'
	let workspaceId = $state(page.url.searchParams.get('workspace_id') || '')
	let clientId = page.url.searchParams.get('client_id') || ''
	let clientName = page.url.searchParams.get('client_name') || 'Unknown Client'
	let redirectUri = page.url.searchParams.get('redirect_uri') || ''
	let scope = $state(page.url.searchParams.get('scope') || 'mcp:all')
	let oauthState = page.url.searchParams.get('state') || ''
	let codeChallenge = page.url.searchParams.get('code_challenge') || ''
	let codeChallengeMethod = page.url.searchParams.get('code_challenge_method') || ''

	let loading = $state(false)
	let success = $state(false)
	let successRedirectUrl = $state('')

	// Gateway mode: fetch user's workspaces for the picker
	let workspaces: Array<{ value: string; label: string }> = $state([])
	let loadingWorkspaces = $state(false)

	if (isGateway) {
		loadingWorkspaces = true
		WorkspaceService.listUserWorkspaces()
			.then((result) => {
				workspaces = result.workspaces
					.filter((w) => !w.disabled)
					.map((w) => ({
						value: w.id,
						label: w.name
					}))
				// Auto-select if only one workspace
				if (workspaces.length === 1) {
					workspaceId = workspaces[0].value
				}
			})
			.catch((e) => {
				sendUserToast('Failed to load workspaces', true)
			})
			.finally(() => {
				loadingWorkspaces = false
			})
	}

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
			sendUserToast('Please select a workspace', true)
			return
		}
		loading = true
		try {
			// Gateway mode uses the gateway approve endpoint with workspace_id in body
			// Workspace mode uses the workspace-scoped approve endpoint
			const approveUrl = isGateway
				? `/api/mcp/gateway/oauth/server/approve`
				: `/api/w/${workspaceId}/mcp/oauth/server/approve`

			const body: Record<string, string> = {
				client_id: clientId,
				redirect_uri: redirectUri,
				scope: scope,
				state: oauthState,
				code_challenge: codeChallenge,
				code_challenge_method: codeChallengeMethod
			}

			// Gateway mode sends workspace_id in the body
			if (isGateway) {
				body.workspace_id = workspaceId
			}

			const response = await fetch(approveUrl, {
				method: 'POST',
				body: JSON.stringify(body),
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

{#if !isGateway && !workspaceId}
	<p class="text-center text-sm text-primary mb-6">Error: missing workspace_id</p>
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
				is requesting access to
				{#if isGateway && !workspaceId}
					your Windmill workspace.
				{:else}
					your
					<span class="font-semibold text-accent">{workspaceId}</span>
					workspace.
				{/if}
			</p>

			{#if isGateway}
				<div class="mb-6">
					<p class="text-xs font-semibold text-emphasis mb-2">Select a workspace:</p>
					{#if loadingWorkspaces}
						<p class="text-xs text-secondary">Loading workspaces...</p>
					{:else if workspaces.length === 0}
						<p class="text-xs text-secondary">No workspaces available.</p>
					{:else}
						<Select items={workspaces} bind:value={workspaceId} placeholder="Choose a workspace" />
					{/if}
				</div>
			{/if}

			{#if workspaceId}
				<div class="mb-6">
					<McpScopeSelector {workspaceId} bind:scope />
				</div>
			{/if}

			<div class="flex flex-row justify-around gap-x-4">
				<Button variant="border" size="lg" disabled={loading} onClick={onDeny}>Deny</Button>
				<Button
					variant="accent"
					size="lg"
					disabled={loading || (isGateway && !workspaceId)}
					{loading}
					onClick={onApprove}>Approve</Button
				>
			</div>
		{/if}
	</CenteredModal>
{/if}
