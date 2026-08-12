<script lang="ts">
	import { Button } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import Password from '$lib/components/Password.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import McpOAuthConnect from './McpOAuthConnect.svelte'
	import OauthScopes from '$lib/components/OauthScopes.svelte'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import { onDestroy, untrack } from 'svelte'
	import { ArrowLeft, ExternalLink, Pen } from 'lucide-svelte'
	import { MCP_REGISTRY, findMcpEntry } from './registry'
	import { OauthService, ResourceService, VariableService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		onConnected: (resourcePath: string) => void
		onCancel: () => void
		workspace?: string
	}

	let { onConnected, onCancel, workspace }: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore!)
	// Any URL is connectable; a suggestion is a shortcut that also pins how the
	// server hands out credentials, which a typed URL cannot tell us.
	let suggested = $state<string | undefined>(undefined)
	let entry = $derived(suggested ? findMcpEntry(suggested) : undefined)

	let instanceConnects = $state<string[] | undefined>(undefined)
	let showOAuth = $state(false)
	let signingIn = $state(false)
	let editScopes = $state(false)
	// Seeded from the instance connect, then left editable: a server may want more
	// than the connect asks for (org-scoped search needs read:org, for instance),
	// and the connect itself is shared with other integrations so it is not widened.
	let scopes = $state<string[]>([])

	let url = $state('')
	let manualToken = $state<string | undefined>(undefined)
	let manualPath = $state('')
	let manualPathError = $state('')
	let saving = $state(false)

	/** Slug for the resource value's `name` and for the path suggestion. Hosts are
	 * reduced to the label that names the service, so mcp.notion.com reads notion. */
	let serverName = $derived.by(() => {
		if (entry) return entry.id
		try {
			const labels = new URL(url).hostname.split('.')
			const named = labels.filter((l) => !['www', 'mcp', 'api'].includes(l))
			return (named[0] ?? labels[0] ?? 'mcp').replace(/[^a-z0-9]/gi, '_')
		} catch {
			return 'mcp'
		}
	})
	let suggestedPath = $derived(
		`u/${$userStore?.username ?? 'user'}/${serverName === 'mcp' ? 'mcp_server' : `${serverName}_mcp`}`
	)
	// Path reads `path` only when it mounts, so the suggestion is applied here and
	// the picker is re-keyed on it. A path the user typed is left alone.
	let lastSuggestion = ''
	$effect(() => {
		const next = suggestedPath
		untrack(() => {
			if (manualPath === '' || manualPath === lastSuggestion) manualPath = next
		})
		lastSuggestion = next
	})

	$effect(() => {
		if (instanceConnects === undefined) {
			OauthService.listOauthConnects()
				.then((l) => (instanceConnects = l.map((x) => x.name)))
				.catch(() => (instanceConnects = []))
		}
	})

	// Which flow this server can actually use here, so the reason a button is
	// missing is visible before it is clicked rather than after it errors.
	let oauthAppReady = $derived(
		entry?.auth === 'oauth_app' &&
			entry.connectClient !== undefined &&
			(instanceConnects?.includes(entry.connectClient) ?? false)
	)
	let needsOauthApp = $derived(entry?.auth === 'oauth_app' && !oauthAppReady)
	let canDiscover = $derived(!needsOauthApp && !oauthAppReady && !!url)

	$effect(() => {
		const client = entry?.connectClient
		if (client && oauthAppReady) {
			OauthService.getOauthConnect({ client })
				.then((c) => (scopes = c.scopes ?? []))
				.catch(() => (scopes = []))
		}
	})

	function pick(id: string) {
		showOAuth = false
		if (suggested === id) {
			// Deselecting leaves the url in place to edit; an edited url is no longer
			// that suggestion, so its auth kind must stop applying with it.
			suggested = undefined
			return
		}
		suggested = id
		url = findMcpEntry(id)?.url ?? url
	}

	async function createMcpResource(path: string, tokenRef: string) {
		await ResourceService.createResource({
			workspace: ws,
			requestBody: {
				resource_type: 'mcp',
				path,
				value: { name: serverName, url, token: tokenRef },
				description: `${entry?.name ?? serverName} MCP server`
			}
		})
		onConnected(path)
	}

	function startProviderOAuth() {
		const client = entry?.connectClient
		if (!client || !manualPath) return
		const connectUrl = new URL(`/api/oauth/connect/${client}`, window.location.origin)
		connectUrl.searchParams.set('scopes', scopes.join('+'))
		if (!window.open(connectUrl.toString(), '_blank', 'popup=true')) {
			sendUserToast('Popup blocked. Allow popups for this site.', true)
			return
		}
		window.addEventListener('message', onOAuthMessage)
		window.addEventListener('storage', onOAuthStorage)
		signingIn = true
	}

	function onOAuthMessage(event: MessageEvent) {
		if (!sameTopDomainOrigin(event.origin, window.location.origin)) return
		if (event.data?.type === 'success') {
			cleanupOAuth()
			void finishProviderOAuth(event.data.res)
		} else if (event.data?.type === 'error') {
			cleanupOAuth()
			signingIn = false
			sendUserToast(event.data.error, true)
		}
	}

	function onOAuthStorage(event: StorageEvent) {
		if (event.key !== 'oauth-callback') return
		cleanupOAuth()
		try {
			const data = JSON.parse(event.newValue || '{}')
			localStorage.removeItem('oauth-callback')
			if (data.type === 'success') {
				void finishProviderOAuth(data.res)
			} else {
				signingIn = false
				sendUserToast(data.error, true)
			}
		} catch (e) {
			signingIn = false
			console.error('Error parsing oauth callback', e)
		}
	}

	function cleanupOAuth() {
		window.removeEventListener('message', onOAuthMessage)
		window.removeEventListener('storage', onOAuthStorage)
	}

	onDestroy(cleanupOAuth)

	/** Store the token like the resource connect does: a secret variable, plus an
	 * account when the provider issues expiring tokens so refresh can run. */
	async function finishProviderOAuth(res: any) {
		try {
			let account: number | undefined = undefined
			if (res?.expires_in != undefined) {
				account = Number(
					await OauthService.createAccount({
						workspace: ws,
						requestBody: {
							refresh_token: res.refresh_token ?? '',
							expires_in: res.expires_in,
							client: entry!.connectClient!,
							scopes
						}
					})
				)
			}
			await VariableService.createVariable({
				workspace: ws,
				requestBody: {
					path: manualPath,
					value: res.access_token,
					is_secret: true,
					is_oauth: true,
					account,
					description: `OAuth token for ${entry!.name}`
				}
			})
			await createMcpResource(manualPath, `$var:${manualPath}`)
			sendUserToast(`Connected ${entry!.name}`)
		} catch (e) {
			sendUserToast(`Failed to connect ${entry?.name}: ${e.body ?? e.message}`, true)
		} finally {
			signingIn = false
		}
	}

	async function saveManual() {
		const token = manualToken
		if (!url || !token || !manualPath) return
		saving = true
		try {
			await VariableService.createVariable({
				workspace: ws,
				requestBody: {
					path: `${manualPath}_token`,
					value: token,
					is_secret: true,
					description: `Token for the ${manualPath} MCP server`
				}
			})
			await createMcpResource(manualPath, `$var:${manualPath}_token`)
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
		<span class="text-sm font-semibold text-emphasis">Connect an MCP server</span>
		<Button unifiedSize="2xs" variant="subtle" onClick={onCancel}>Cancel</Button>
	</div>

	<div class="flex flex-col gap-1">
		<Label label="MCP server URL">
			<TextInput
				inputProps={{
					type: 'url',
					placeholder: 'https://mcp.example.com',
					disabled: entry !== undefined
				}}
				bind:value={url}
			/>
		</Label>
		<div class="flex flex-row flex-wrap items-center gap-1">
			<span class="text-2xs text-secondary mr-1">Suggested</span>
			{#each MCP_REGISTRY as e (e.id)}
				<Button
					unifiedSize="2xs"
					variant="subtle"
					selected={suggested === e.id}
					startIcon={{ icon: e.icon, props: { width: '12px', height: '12px' } }}
					onClick={() => pick(e.id)}
				>
					{e.name}
				</Button>
			{/each}
		</div>
	</div>

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

	{#key suggestedPath}
		<Path
			bind:path={manualPath}
			bind:error={manualPathError}
			initialPath={suggestedPath}
			namePlaceholder={serverName}
			kind="resource"
			workspaceOverride={ws}
		/>
	{/key}

	{#if showOAuth}
		<McpOAuthConnect
			server={{ name: entry?.name ?? serverName, url }}
			path={manualPath}
			workspace={ws}
			onConnected={(path) => onConnected(path)}
		/>
		<Button
			unifiedSize="2xs"
			variant="subtle"
			wrapperClasses="self-start"
			startIcon={{ icon: ArrowLeft }}
			onClick={() => (showOAuth = false)}
		>
			Back
		</Button>
	{:else}
		{#if oauthAppReady && entry}
			<div class="text-2xs text-secondary">
				This instance has a {entry.name} OAuth app configured, so you can sign in with your own account.
			</div>
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis flex gap-2 items-center">
					Scopes
					<Button
						unifiedSize="2xs"
						variant="subtle"
						iconOnly
						title="Edit scopes"
						startIcon={{ icon: Pen }}
						onClick={() => (editScopes = !editScopes)}
					/>
				</span>
				{#if editScopes}
					<OauthScopes bind:scopes />
				{:else}
					<div class="flex flex-col gap-1">
						{#each scopes as scope}
							<div class="py-0.5 pl-2 text-xs">- {scope}</div>
						{/each}
					</div>
				{/if}
			</div>
			<Button
				unifiedSize="sm"
				variant="accent"
				wrapperClasses="self-start"
				onClick={startProviderOAuth}
				disabled={signingIn || !manualPath || manualPathError !== ''}
			>
				{signingIn ? 'Finish in the popup...' : `Sign in with ${entry.name}`}
			</Button>
		{:else if needsOauthApp && entry}
			<div class="text-2xs text-secondary">
				{entry.name} does not support dynamic client registration, so signing in needs a {entry.name}
				OAuth app registered on this instance. None is configured, so connect with a token below. An
				admin can add one in instance settings to enable sign-in.
			</div>
		{:else if canDiscover}
			{#if $enterpriseLicense}
				<Button
					unifiedSize="sm"
					variant="accent"
					wrapperClasses="self-start"
					disabled={!manualPath || manualPathError !== ''}
					onClick={() => (showOAuth = true)}
				>
					Sign in with {entry?.name ?? 'OAuth'}
				</Button>
			{:else}
				<div class="text-2xs text-secondary">
					Signing in to an MCP server is an enterprise feature. Connect with a token below instead.
				</div>
			{/if}
		{/if}

		<div class="flex flex-col gap-3 border-t pt-3">
			<span class="text-2xs uppercase tracking-wide text-secondary">Connect with a token</span>
			{#if entry?.tokenHint}
				<div class="text-2xs text-secondary">{entry.tokenHint}</div>
			{/if}
			<Label label="Token">
				<Password bind:password={manualToken} />
			</Label>
			<Button
				unifiedSize="sm"
				wrapperClasses="self-start"
				onClick={saveManual}
				disabled={saving || !url || !manualToken || !manualPath || manualPathError !== ''}
			>
				Save
			</Button>
		</div>
	{/if}
</div>
