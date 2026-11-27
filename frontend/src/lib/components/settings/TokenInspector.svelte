<script lang="ts">
	import { untrack } from 'svelte'
	import { OpenAPI, type TokenIntrospection } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { Alert, Badge, Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import { Check, ShieldCheck, X } from 'lucide-svelte'

	interface Props {
		/** The token to inspect. When unset, the user pastes one. */
		token?: string
		/** Workspace the identity and the route are evaluated in. */
		workspace?: string
		/** Route to check, relative to `/api/w/{workspace}/`, or a full `/api/...` path. */
		defaultPath?: string
		defaultMethod?: string
	}

	let { token, workspace, defaultPath, defaultMethod }: Props = $props()

	let pastedToken = $state('')
	let effectiveToken = $derived((token ?? pastedToken).trim())
	let routeMethod = $state(untrack(() => defaultMethod ?? 'GET'))
	let routePath = $state(untrack(() => defaultPath ?? ''))
	let result = $state<TokenIntrospection | undefined>(undefined)
	let error = $state<string | undefined>(undefined)
	let loading = $state(false)

	const methods = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE'].map((m) => ({ label: m, value: m }))

	const kindLabel: Record<TokenIntrospection['kind'], string> = {
		session: 'Browser session',
		api_token: 'API token',
		jwt: 'Signed token (JWT)',
		external: 'External issuer',
		superadmin_secret: 'Superadmin secret'
	}

	async function inspect() {
		if (!effectiveToken) return
		loading = true
		error = undefined
		result = undefined
		try {
			const params = new URLSearchParams()
			if (workspace) params.set('workspace', workspace)
			if (routePath.trim()) {
				params.set('path', routePath.trim())
				params.set('method', routeMethod)
			}
			// The route describes only the credential presenting the request, so the token
			// must be the bearer credential, not a parameter.
			const response = await fetch(`${OpenAPI.BASE}/tokens/introspect?${params}`, {
				headers: { Authorization: `Bearer ${effectiveToken}` },
				credentials: 'omit'
			})
			if (response.status === 401) {
				error = 'This token is not valid: it does not exist, has expired or was revoked.'
			} else if (!response.ok) {
				error = (await response.text()) || `Request failed (${response.status})`
			} else {
				result = await response.json()
			}
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		} finally {
			loading = false
		}
	}

	/** Whether the server read the token's expiry and workspace bounds: it does for its own rows
	 * and signed tokens, not for an external issuer's or a guest's, whose absence means nothing. */
	function metadataKnown(r: TokenIntrospection): boolean {
		return r.kind === 'api_token' || r.kind === 'session' || r.expiration != undefined
	}

	function expiryText(expiration: string | undefined): string {
		return expiration ? displayDate(expiration) : 'Never'
	}
</script>

<div class="flex flex-col gap-3">
	{#if token === undefined}
		<div class="flex flex-col gap-1">
			<span class="text-xs text-emphasis font-semibold">Token</span>
			<TextInput
				bind:value={pastedToken}
				inputProps={{ type: 'password', placeholder: 'Paste a token to test', autocomplete: 'off' }}
				class="w-full"
			/>
		</div>
	{/if}
	<div class="flex flex-col gap-1">
		<span class="text-xs text-emphasis font-semibold">
			Route to check <span class="text-hint font-normal">(optional)</span>
		</span>
		<div class="flex gap-2 items-center">
			<div class="w-28 shrink-0">
				<Select items={methods} bind:value={routeMethod} size="sm" />
			</div>
			<TextInput
				bind:value={routePath}
				inputProps={{
					placeholder: workspace
						? 'scripts/list, or a full /api/... path'
						: '/api/w/<workspace>/...'
				}}
				size="sm"
				class="w-full"
			/>
			<Button
				variant="accent-secondary"
				unifiedSize="sm"
				startIcon={{ icon: ShieldCheck }}
				disabled={!effectiveToken}
				{loading}
				onClick={inspect}
			>
				Test token
			</Button>
		</div>
	</div>

	{#if error}
		<Alert type="error" title="Could not test the token" size="xs">{error}</Alert>
	{/if}

	{#if result}
		<div class="flex flex-col gap-3 border rounded-md p-3 bg-surface-tertiary text-xs">
			<div class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5">
				<span class="text-secondary">Kind</span>
				<span class="text-primary">{kindLabel[result.kind] ?? result.kind}</span>
				<span class="text-secondary">Acts as</span>
				<span class="text-primary">{result.email} ({result.username})</span>
				{#if result.label}
					<span class="text-secondary">Label</span>
					<span class="text-primary">{result.label}</span>
				{/if}
				{#if result.token_prefix}
					<span class="text-secondary">Prefix</span>
					<span class="text-primary">{result.token_prefix}****</span>
				{/if}
				<span class="text-secondary">Expires</span>
				<span class="text-primary">
					{metadataKnown(result) ? expiryText(result.expiration) : 'Not reported'}
				</span>
				<span class="text-secondary">Workspaces</span>
				<span class="text-primary">
					{#if result.workspace_ids?.length}
						{result.workspace_ids.join(', ')}
					{:else if result.workspace_id}
						{result.workspace_id}
					{:else if metadataKnown(result)}
						Every workspace the owner is a member of
					{:else}
						Not reported
					{/if}
				</span>
				<span class="text-secondary">Rights</span>
				<div class="flex flex-wrap gap-1">
					{#if result.super_admin}
						<Badge color="red" small>Superadmin</Badge>
					{/if}
					{#if result.admin_withheld}
						<Badge color="gray" small>Admin rights withheld</Badge>
					{/if}
					{#if result.read_only}
						<Badge color="blue" small>Read-only</Badge>
					{/if}
					{#if result.job_id}
						<Badge color="gray" small>Job {result.job_id}</Badge>
					{/if}
					{#if !result.super_admin && !result.admin_withheld && !result.read_only && !result.job_id}
						<span class="text-primary">Owner's member rights</span>
					{/if}
				</div>
				{#if result.allowed_ips?.length}
					<span class="text-secondary">Usable from</span>
					<span class="text-primary">{result.allowed_ips.join(', ')}</span>
				{/if}
				<span class="text-secondary">Scopes</span>
				<div class="flex flex-wrap gap-1">
					{#if result.scopes}
						{#each result.scopes as scope (scope)}
							<Badge color="gray" small>{scope}</Badge>
						{:else}
							<span class="text-primary">None</span>
						{/each}
					{:else}
						<span class="text-primary">Unrestricted</span>
					{/if}
				</div>
			</div>

			{#if result.workspace}
				{@const ws = result.workspace}
				<div class="border-t pt-3 grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5">
					<span class="text-secondary">In {ws.workspace_id}</span>
					{#if ws.usable}
						<div class="flex flex-wrap gap-1">
							<Badge color={ws.is_admin ? 'indigo' : ws.is_operator ? 'yellow' : 'green'} small>
								{ws.is_admin ? 'Admin' : ws.is_operator ? 'Restricted role' : 'Member'}
							</Badge>
						</div>
						<span class="text-secondary">Groups</span>
						<span class="text-primary">{ws.groups?.join(', ') || 'None'}</span>
						<span class="text-secondary">Folders</span>
						<span class="text-primary">
							{#if ws.folders?.length}
								{ws.folders
									.map((f) => `${f.name} (${f.owner ? 'owner' : f.write ? 'write' : 'read'})`)
									.join(', ')}
							{:else}
								None
							{/if}
						</span>
					{:else}
						<span class="text-primary">Not usable in this workspace</span>
					{/if}
				</div>
			{/if}

			{#if result.route}
				{@const route = result.route}
				<div class="border-t pt-3 flex flex-col gap-1">
					<div class="flex items-start gap-2">
						{#if route.allowed}
							<Check size={14} class="shrink-0 mt-0.5 text-green-600 dark:text-green-400" />
						{:else}
							<X size={14} class="shrink-0 mt-0.5 text-red-600 dark:text-red-400" />
						{/if}
						<span class="text-primary font-mono break-all">{route.method} {route.path}</span>
					</div>
					{#if route.allowed}
						<span class="text-secondary">
							The token passes the route checks. The endpoint can still refuse it: a scope that
							names a path is matched against the item, and the identity needs permission on it.
						</span>
					{:else}
						<span class="text-secondary">{route.reason}</span>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>
