<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import Path from '$lib/components/Path.svelte'
	import AppConnectDrawer from '$lib/components/AppConnectDrawer.svelte'
	import McpOAuthConnect from './McpOAuthConnect.svelte'
	import { MCP_REGISTRY, findMcpEntry } from './registry'
	import { OauthService, ResourceService, VariableService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { ExternalLink } from 'lucide-svelte'

	interface Props {
		onConnected: (resourcePath: string) => void
		onCancel: () => void
		workspace?: string
	}

	let { onConnected, onCancel, workspace }: Props = $props()

	const CUSTOM = '__custom__'

	let ws = $derived(workspace ?? $workspaceStore!)
	let selected = $state<string>(MCP_REGISTRY[0]?.id ?? CUSTOM)
	let entry = $derived(selected === CUSTOM ? undefined : findMcpEntry(selected))

	let instanceConnects = $state<string[] | undefined>(undefined)
	let connectScopes = $state<string[] | undefined>(undefined)
	let appConnectDrawer: AppConnectDrawer | undefined = $state(undefined)
	let showOAuth = $state(false)

	// Manual path: a URL and a token typed by hand.
	let manualUrl = $state('')
	let manualToken = $state('')
	let manualPath = $state('')
	let manualPathError = $state('')
	// Path derives its own suggestion from `namePlaceholder` only at mount, which
	// leaves the previous server's name behind when the selection changes; drive
	// the suggestion from the chosen server instead.
	let suggestedPath = $derived(`u/${$userStore?.username ?? 'user'}/${entry?.id ?? 'mcp'}_mcp`)
	$effect(() => {
		manualPath = suggestedPath
	})
	let saving = $state(false)

	$effect(() => {
		if (instanceConnects === undefined) {
			OauthService.listOauthConnects()
				.then((l) => (instanceConnects = l.map((x) => x.name)))
				.catch(() => (instanceConnects = []))
		}
	})

	// The token is minted with whatever the instance connect asks for, so a
	// connect that is missing a scope produces tools that fail one by one at call
	// time instead of failing here.
	$effect(() => {
		const client = entry?.connectClient
		if (client && oauthAppReady) {
			OauthService.getOauthConnect({ client })
				.then((c) => (connectScopes = c.scopes ?? []))
				.catch(() => (connectScopes = undefined))
		}
	})

	let missingScopes = $derived(
		connectScopes === undefined
			? []
			: (entry?.requiredScopes ?? []).filter((sc) => !connectScopes!.includes(sc))
	)

	// Which flows this server can actually use here, so the reason a button is
	// missing is visible before it is clicked rather than after it errors.
	let oauthAppReady = $derived(
		entry?.auth === 'oauth_app' &&
			entry.connectClient !== undefined &&
			(instanceConnects?.includes(entry.connectClient) ?? false)
	)
	let dcrReady = $derived(entry?.auth === 'dcr' && !!$enterpriseLicense)

	let items = $derived([
		...MCP_REGISTRY.map((e) => ({ label: e.name, value: e.id })),
		{ label: 'Other (enter a URL)', value: CUSTOM }
	])

	async function createMcpResource(path: string, name: string, url: string, tokenRef: string) {
		await ResourceService.createResource({
			workspace: ws,
			requestBody: {
				resource_type: 'mcp',
				path,
				value: { name, url, token: tokenRef },
				description: `${name} MCP server`
			}
		})
		onConnected(path)
	}

	// The OAuth connect leaves the credential in a secret variable and points the
	// provider resource's `token` at it; reuse that reference so the MCP resource
	// rides the same variable (and, for OAuth, the account that refreshes it).
	async function onProviderConnected(providerPath: string) {
		if (!entry) return
		try {
			const resource = await ResourceService.getResource({ workspace: ws, path: providerPath })
			if (resource.resource_type !== entry.connectClient) {
				throw new Error(
					`${providerPath} is a ${resource.resource_type} resource, not ${entry.connectClient}`
				)
			}
			const token = (resource.value as { token?: unknown } | undefined)?.token
			if (typeof token !== 'string' || token === '') {
				throw new Error(`No token found on ${providerPath}`)
			}
			let tokenRef = token
			if (!token.startsWith('$var:')) {
				const varPath = `${providerPath}_mcp_token`
				await VariableService.createVariable({
					workspace: ws,
					requestBody: {
						path: varPath,
						value: token,
						is_secret: true,
						description: `${entry.name} token for the ${providerPath}_mcp MCP server`
					}
				})
				tokenRef = `$var:${varPath}`
			}
			await createMcpResource(`${providerPath}_mcp`, entry.id, entry.url, tokenRef)
			sendUserToast(`Connected ${entry.name}`)
		} catch (e) {
			sendUserToast(`Failed to connect ${entry.name}: ${e.body ?? e.message}`, true)
		}
	}

	async function saveManual() {
		const url = entry?.url ?? manualUrl
		const name = entry?.id ?? 'mcp'
		if (!url || !manualToken || !manualPath) return
		saving = true
		try {
			await VariableService.createVariable({
				workspace: ws,
				requestBody: {
					path: `${manualPath}_token`,
					value: manualToken,
					is_secret: true,
					description: `Token for the ${manualPath} MCP server`
				}
			})
			await createMcpResource(manualPath, name, url, `$var:${manualPath}_token`)
			sendUserToast(`Connected ${entry?.name ?? url}`)
		} catch (e) {
			sendUserToast(`Failed to connect: ${e.body ?? e.message}`, true)
		} finally {
			saving = false
		}
	}
</script>

<div class="border rounded p-4 bg-surface-secondary flex flex-col gap-4">
	<div class="flex justify-between items-center">
		<span class="font-semibold text-sm">Connect an MCP server</span>
		<Button size="xs" color="light" onClick={onCancel}>Cancel</Button>
	</div>

	<Label label="Server">
		<Select {items} bind:value={selected} clearable={false} />
	</Label>

	{#if entry?.docsUrl}
		<a
			href={entry.docsUrl}
			target="_blank"
			rel="noopener noreferrer"
			class="text-2xs text-secondary inline-flex items-center gap-1"
		>
			{entry.name} MCP documentation <ExternalLink size={12} />
		</a>
	{/if}

	{#if showOAuth && entry}
		<McpOAuthConnect
			preset={{ name: entry.name, url: entry.url }}
			workspace={ws}
			onConnected={(path) => onConnected(path)}
			onCancel={() => (showOAuth = false)}
		/>
	{:else if selected === CUSTOM && $enterpriseLicense}
		<McpOAuthConnect workspace={ws} onConnected={(path) => onConnected(path)} {onCancel} />
	{:else}
		{#if entry?.auth === 'oauth_app'}
			{#if oauthAppReady}
				<div class="text-2xs text-secondary">
					This instance has a {entry.name} OAuth app configured, so you can sign in with your own account.
				</div>
				{#if missingScopes.length > 0}
					<Alert type="warning" title="The {entry.name} connect is missing scopes" size="xs">
						Add {missingScopes.join(', ')} to the {entry.connectClient} OAuth connect in instance
						settings, otherwise some tools are refused. Reconnect after changing them — an existing
						token keeps the scopes it was granted.
					</Alert>
				{/if}
				<Button
					size="sm"
					onClick={() => appConnectDrawer?.open(entry?.connectClient)}
					disabled={instanceConnects === undefined}
				>
					Sign in with {entry.name}
				</Button>
			{:else}
				<div class="text-2xs text-secondary">
					{entry.name} does not support dynamic client registration, so signing in needs a
					{entry.name} OAuth app registered on this instance. None is configured, so connect with a token
					below. An admin can add one in instance settings to enable sign-in.
				</div>
			{/if}
		{:else if entry?.auth === 'dcr'}
			{#if dcrReady}
				<Button size="sm" onClick={() => (showOAuth = true)}>Sign in with {entry.name}</Button>
			{:else}
				<div class="text-2xs text-secondary">
					Signing in to an MCP server is an enterprise feature. Connect with a token below instead.
				</div>
			{/if}
		{/if}

		<div class="flex flex-col gap-3 border-t pt-3">
			<span class="text-2xs uppercase tracking-wide text-secondary">Connect with a token</span>
			{#if entry?.tokenHint}
				<div class="text-2xs text-tertiary">{entry.tokenHint}</div>
			{/if}
			{#if selected === CUSTOM}
				<Label label="MCP server URL">
					<input
						type="url"
						bind:value={manualUrl}
						placeholder="https://mcp.example.com"
						class="text-sm w-full"
					/>
				</Label>
			{/if}
			<Label label="Token">
				<input type="password" bind:value={manualToken} class="text-sm w-full" />
			</Label>
			{#key selected}
				<Path
					bind:path={manualPath}
					bind:error={manualPathError}
					initialPath={suggestedPath}
					namePlaceholder={entry?.id ?? 'mcp'}
					kind="resource"
					workspaceOverride={ws}
				/>
			{/key}
			<Button
				size="sm"
				onClick={saveManual}
				disabled={saving ||
					!manualToken ||
					!manualPath ||
					manualPathError !== '' ||
					(selected === CUSTOM && !manualUrl)}
			>
				Save
			</Button>
		</div>
	{/if}
</div>

<AppConnectDrawer
	bind:this={appConnectDrawer}
	workspace={ws}
	on:refresh={(e) => onProviderConnected(e.detail)}
/>
