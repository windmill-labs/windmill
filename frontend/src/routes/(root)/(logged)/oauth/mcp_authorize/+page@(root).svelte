<!-- `@(root)` skips the (logged) layout on purpose: that layout renders nothing but
     "Loading user..." until `$userStore` is set, and `$userStore` is only ever filled in
     for a chosen workspace. An MCP client sends the browser straight here after login, so
     in gateway mode there is no workspace yet — picking one is what this page is for. -->
<script lang="ts">
	import { page } from '$app/state'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
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
	// The server the client connects to: the full experience (`/api/mcp`) takes an access level
	// rather than a selection of scripts and flows.
	const isFullExperience = (() => {
		try {
			return new URL(page.url.searchParams.get('resource') ?? '').pathname === '/api/mcp'
		} catch {
			return false
		}
	})()
	let access = $state<'read' | 'read_run' | 'full'>('full')
	const ACCESS_SCOPES = {
		read: 'mcp:all mcp:read_only',
		read_run: 'mcp:all mcp:read_run',
		full: 'mcp:all'
	}
	let oauthState = page.url.searchParams.get('state') || ''
	let codeChallenge = page.url.searchParams.get('code_challenge') || ''
	let codeChallengeMethod = page.url.searchParams.get('code_challenge_method') || ''

	// Defense-in-depth (GHSA-q9xg-f2v2-695g): the backend validates redirect_uris at
	// registration, but this page navigates the browser to `redirectUri`, so reject
	// browser-executable schemes here too — this also neutralizes any client rows
	// registered before the backend fix.
	const DISALLOWED_REDIRECT_SCHEMES = ['javascript:', 'data:', 'vbscript:', 'blob:', 'file:']
	function isSafeRedirectUri(uri: string): boolean {
		try {
			return !DISALLOWED_REDIRECT_SCHEMES.includes(new URL(uri).protocol.toLowerCase())
		} catch {
			return false
		}
	}
	let redirectUriValid = isSafeRedirectUri(redirectUri)

	let loading = $state(false)
	let success = $state(false)
	let successRedirectUrl = $state('')

	// Gateway mode: fetch user's workspaces for the picker. One picked workspace binds the
	// token to it; several, or all of them, leave it unbound and its tools name their workspace.
	let workspaces: Array<{ value: string; label: string }> = $state([])
	let pickedWorkspaces = $state<string[]>([])
	let allWorkspaces = $state(false)
	let loadingWorkspaces = $state(false)
	const grantsWorkspace = $derived(!isGateway || allWorkspaces || pickedWorkspaces.length > 0)
	// The workspace the scope picker browses scripts and flows in.
	const scopeWorkspaceId = $derived(
		isGateway ? (pickedWorkspaces[0] ?? (allWorkspaces ? workspaces[0]?.value : '')) : workspaceId
	)

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
					pickedWorkspaces = [workspaces[0].value]
				}
			})
			.catch(() => {
				sendUserToast('Failed to load workspaces', true)
			})
			.finally(() => {
				loadingWorkspaces = false
			})
	}

	function onDeny() {
		if (!redirectUriValid) return
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
		if (!redirectUriValid) return
		if (!grantsWorkspace) {
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

			const body: Record<string, unknown> = {
				client_id: clientId,
				redirect_uri: redirectUri,
				scope: isFullExperience ? ACCESS_SCOPES[access] : scope,
				state: oauthState,
				code_challenge: codeChallenge,
				code_challenge_method: codeChallengeMethod
			}

			// Gateway mode sends the workspaces in the body
			if (isGateway) {
				body.workspace_ids = allWorkspaces ? [] : pickedWorkspaces
				body.all_workspaces = allWorkspaces
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

{#if !redirectUriValid}
	<CenteredModal title="Authorization Request">
		<p class="text-center text-sm text-primary"> Error: invalid or unsafe redirect_uri </p>
	</CenteredModal>
{:else if !isGateway && !workspaceId}
	<CenteredModal title="Authorization Request">
		<p class="text-center text-sm text-primary">Error: missing workspace_id</p>
	</CenteredModal>
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
				{#if isGateway}
					your Windmill workspaces.
				{:else}
					your
					<span class="font-semibold text-accent">{workspaceId}</span>
					workspace.
				{/if}
			</p>

			{#if isGateway}
				<div class="mb-6 flex flex-col gap-2">
					<p class="text-xs font-semibold text-emphasis">Workspaces it may work in:</p>
					{#if loadingWorkspaces}
						<p class="text-xs text-secondary">Loading workspaces...</p>
					{:else if workspaces.length === 0}
						<p class="text-xs text-secondary">No workspaces available.</p>
					{:else}
						<Toggle
							bind:checked={allWorkspaces}
							options={{ right: 'All my workspaces, including ones I join later' }}
							size="xs"
						/>
						{#if !allWorkspaces}
							<MultiSelect
								items={workspaces}
								bind:value={pickedWorkspaces}
								placeholder="Choose workspaces"
								reorderable={false}
							/>
						{/if}
					{/if}
				</div>
			{/if}

			{#if isFullExperience}
				<div class="mb-6 flex flex-col gap-2">
					<p class="text-xs font-semibold text-emphasis">What it may do:</p>
					<ToggleButtonGroup bind:selected={access} allowEmpty={false}>
						{#snippet children({ item })}
							<ToggleButton
								{item}
								value="read"
								label="Read only"
								tooltip="Explore and read. Nothing is run or changed."
							/>
							<ToggleButton
								{item}
								value="read_run"
								label="Read and run"
								tooltip="Explore, and run deployed scripts and flows. Nothing is changed."
							/>
							<ToggleButton
								{item}
								value="full"
								label="Full"
								tooltip="Everything you can do yourself, writes included."
							/>
						{/snippet}
					</ToggleButtonGroup>
				</div>
			{:else if scopeWorkspaceId}
				<div class="mb-6">
					<McpScopeSelector workspaceId={scopeWorkspaceId} bind:scope />
				</div>
			{/if}

			<div class="flex flex-row justify-around gap-x-4">
				<Button variant="border" size="lg" disabled={loading} onClick={onDeny}>Deny</Button>
				<Button
					variant="accent"
					size="lg"
					disabled={loading || !grantsWorkspace}
					{loading}
					onClick={onApprove}>Approve</Button
				>
			</div>
		{/if}
	</CenteredModal>
{/if}
